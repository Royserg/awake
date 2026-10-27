import type { Clock, PresentationDriver, PresentationRequest, StartupTuning, VisualMode } from './types.ts';
import { DEFAULT_TUNING, REDUCED_INTRO_MS } from './tuning.ts';

export interface NativePort {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, callback: (payload: T) => void): Promise<() => void>;
}

export interface NativeReceipt {
  runId: string;
  compactToken: string | null;
  revision: number;
  status: 'preparing' | 'active' | 'ended' | 'failed';
  error: string | null;
}

interface Entry {
  request: PresentationRequest;
  mode: PresentationRequest['mode'];
  status: NativeReceipt['status'];
  nativeRevision: number;
  token?: string;
  resolve: () => void;
  reject: (reason: unknown) => void;
  ready: Promise<void>;
  complete: () => void;
  controls: Array<Record<string, unknown>>;
}

export interface NativePresentationOptions {
  scope: 'production' | 'preview';
  clock: Clock;
  tuning?: Readonly<StartupTuning>;
  reducedMotion?: boolean;
  prepare?: (mode: VisualMode) => Promise<void>;
  visible?: (mode: VisualMode) => void;
  onUserHide?: () => void;
  onCompatibilityShow?: () => void;
  onStatus?: (status: NativeReceipt['status'], error?: string | null) => void;
  onSample?: (sample: { elapsedMs: number; paused: boolean }) => void;
}

function createReadiness() {
  let resolve!: () => void;
  let reject!: (reason: unknown) => void;
  const ready = new Promise<void>((yes, no) => { resolve = yes; reject = no; });
  // A native event may reject before its command response reaches the owner.
  void ready.catch(() => {});
  return { ready, resolve, reject };
}

/** Pure IPC coordinator: native revisions and tokens are never inferred from labels. */
export class NativePresentation implements PresentationDriver {
  private readonly port: NativePort;
  private readonly options: NativePresentationOptions;
  private readonly initialized: Promise<void>;
  private readonly unlisten: Array<() => void> = [];
  private nativeRevision = 0;
  private localRevision = 0;
  private entry?: Entry;
  private cancelStatic?: () => void;
  private disposed = false;
  private initializedOk = false;
  private previewSequence = 0;

  constructor(port: NativePort, options: NativePresentationOptions) {
    this.port = port;
    this.options = options;
    this.initialized = this.initialize();
    void this.initialized.catch(() => {});
  }

  private async initialize() {
    this.unlisten.push(await this.port.listen<NativeReceipt>(
      'startup-presentation', receipt => this.receipt(receipt),
    ));
    if (this.options.scope === 'preview') {
      this.unlisten.push(await this.port.listen<{ runId: string; elapsedMs: number; paused: boolean }>(
        'startup-preview-sample', sample => {
          if (this.entry?.status === 'active' && this.entry.token === sample.runId) {
            this.options.onSample?.(sample);
          }
        },
      ));
    } else {
      this.unlisten.push(await this.port.listen('startup-main-hidden', () => this.options.onUserHide?.()));
      this.unlisten.push(await this.port.listen('startup-compat-show', () => this.options.onCompatibilityShow?.()));
    }
    const context = this.options.scope === 'production' ? 'startup_context' : 'preview_context';
    this.nativeRevision = await this.port.invoke<number>(context);
    this.initializedOk = true;
  }

  configure(tuning: Readonly<StartupTuning>, reducedMotion: boolean) {
    this.options.tuning = tuning;
    this.options.reducedMotion = reducedMotion;
  }

  nextPreviewRevision() { return ++this.previewSequence; }

  private receipt(receipt: NativeReceipt) {
    const entry = this.entry;
    if (!entry || receipt.revision !== entry.nativeRevision || entry.request.revision !== this.localRevision) return;
    if (entry.status === 'ended' || entry.status === 'failed') return;
    if (entry.status === 'active' && (receipt.status === 'preparing' || receipt.status === 'active')) return;

    entry.token = receipt.runId;
    entry.status = receipt.status;
    this.options.onStatus?.(receipt.status, receipt.error);
    if (receipt.status === 'active') {
      this.options.visible?.(entry.mode);
      entry.resolve();
      for (const control of entry.controls.splice(0)) void this.control(control).catch(() => {});
    } else if (receipt.status === 'ended') {
      entry.resolve();
      this.options.visible?.('hidden');
      entry.complete();
    } else if (receipt.status === 'failed') {
      entry.reject(new Error(receipt.error ?? 'Native presentation failed'));
    }
  }

  async present(request: PresentationRequest, complete: () => void): Promise<void> {
    if (this.disposed || request.revision < this.localRevision) return;
    if (request.revision === this.localRevision) {
      const existing = this.entry;
      if (!existing || request.mode !== existing.request.mode || request.durationMs !== existing.request.durationMs) {
        throw new Error('revision_conflict');
      }
      return existing.ready;
    }

    this.entry?.resolve();
    this.cancelStatic?.();
    this.localRevision = request.revision;
    const entry: Entry = {
      request: { ...request }, mode: request.mode, status: 'preparing', nativeRevision: 0,
      ...createReadiness(), complete, controls: [],
    };
    this.entry = entry;
    try {
      await this.initialized;
      if (!this.current(entry)) return;
      await this.options.prepare?.(request.mode);
      if (!this.current(entry)) return;
      await this.issue(entry, request.mode);
      await entry.ready;
    } catch (error) {
      if (!this.current(entry)) return;
      this.options.onStatus?.('failed', String(error));
      if (!this.initializedOk || request.mode !== 'cinematic' || this.options.scope !== 'production') throw error;
      // Fail closed: never substitute an opaque monitor-sized window.
      await this.options.prepare?.('compact-intro');
      if (!this.current(entry)) return;
      entry.mode = 'compact-intro';
      Object.assign(entry, createReadiness());
      await this.issue(entry, 'compact-intro');
      await entry.ready;
    }

    if (this.current(entry) && entry.mode === 'compact-intro' && this.options.scope === 'production') {
      this.cancelStatic = this.options.clock.schedule(REDUCED_INTRO_MS, () => {
        if (this.current(entry)) complete();
      });
    }
  }

  private current(entry: Entry) {
    return !this.disposed && this.entry === entry && this.localRevision === entry.request.revision;
  }

  private async issue(entry: Entry, mode: PresentationRequest['mode']) {
    entry.status = 'preparing';
    entry.nativeRevision = ++this.nativeRevision;
    const preview = this.options.scope === 'preview';
    const command = preview ? 'preview_present' : mode === 'cinematic' ? 'start_intro' : 'show_compact';
    const args: Record<string, unknown> = { revision: entry.nativeRevision };
    if (preview) {
      Object.assign(args, {
        kind: mode, tuning: this.options.tuning ?? DEFAULT_TUNING,
        reducedMotion: this.options.reducedMotion ?? false,
      });
    } else if (mode === 'cinematic') {
      args.tuning = this.options.tuning ?? DEFAULT_TUNING;
    } else {
      args.kind = mode === 'compact-intro' ? 'intro' : mode === 'compact-wait' ? 'wait' : 'error';
    }

    const receipt = await this.port.invoke<NativeReceipt>(command, args);
    if (!this.current(entry)) {
      // Token-only cleanup cannot cancel a newer run in the same fixed-label window.
      const finish = preview ? 'preview_finish' : mode === 'cinematic' ? 'finish_intro' : 'hide_compact';
      const identity = preview || mode === 'cinematic'
        ? { runId: receipt.runId, revision: null }
        : { compactToken: receipt.compactToken, revision: null };
      await this.port.invoke(finish, identity);
      return;
    }
    this.receipt(receipt);
  }

  async hide(revision: number) {
    if (revision < this.localRevision || (revision === this.localRevision && !this.entry)) return;
    this.localRevision = revision;
    this.entry?.resolve();
    this.entry = undefined;
    this.cancelStatic?.();
    this.options.visible?.('hidden');
    await this.initialized;
    if (revision !== this.localRevision) return;

    const barrier = ++this.nativeRevision;
    if (this.options.scope === 'preview') {
      await this.port.invoke('preview_finish', { runId: null, revision: barrier });
    } else {
      await this.port.invoke('finish_intro', { runId: null, revision: barrier });
      if (revision === this.localRevision) {
        await this.port.invoke('hide_compact', { compactToken: null, revision: barrier });
      }
    }
    if (revision === this.localRevision) this.options.onStatus?.('ended');
  }

  async control(control: Record<string, unknown>) {
    if (this.options.scope !== 'preview' || this.disposed) return;
    const entry = this.entry;
    if (!entry || entry.status === 'ended' || entry.status === 'failed') return;
    if (!entry.token || entry.status !== 'active') {
      entry.controls.push(control);
      return;
    }
    await this.port.invoke('preview_control', { runId: entry.token, control });
  }

  async dispose() {
    if (this.disposed) return;
    const hiding = this.hide(this.localRevision + 1);
    this.disposed = true;
    try {
      await hiding;
    } finally {
      for (const unlisten of this.unlisten.splice(0)) unlisten();
    }
  }
}
