import { StartupController } from './controller.ts';
import type { Clock, DownloadProgress, PresentationDriver, PresentationRequest, StartupTuning, UpdateDriver, UpdateSession, VisualMode } from './types.ts';
import { DEFAULT_TUNING, introDuration, REDUCED_INTRO_MS, validateTuning } from './tuning.ts';
import type { NativePresentation } from './native-presentation.ts';

/** Deterministic virtual time: seeking a visual never dispatches update events. */
export class VirtualClock implements Clock {
  private time = 0;
  private nextId = 0;
  private tasks = new Map<number, { at: number; callback: () => void }>();
  now() { return this.time; }
  get pendingCount() { return this.tasks.size; }
  schedule(delayMs: number, callback: () => void) {
    const id = ++this.nextId;
    this.tasks.set(id, { at: this.time + Math.max(0, delayMs), callback });
    return () => { this.tasks.delete(id); };
  }
  advance(deltaMs: number) {
    const end = this.time + Math.max(0, deltaMs);
    let task;
    while ((task = [...this.tasks.entries()].filter(([, t]) => t.at <= end).sort((a, b) => a[1].at - b[1].at || a[0] - b[0])[0])) {
      this.time = task[1].at; this.tasks.delete(task[0]); task[1].callback();
    }
    this.time = end;
  }
  clear() { this.tasks.clear(); }
}

export const PREVIEW_SCENARIOS = {
  ordinary: 'Ordinary launch', slow: 'Slow update check', early: 'Update at 300 ms',
  late: 'Update after intro', download: 'Long download', install: 'Installation',
  'download-error': 'Download error', 'install-error': 'Installation error', 'restart-error': 'Restart error',
} as const;
export type PreviewScenario = keyof typeof PREVIEW_SCENARIOS;

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void, reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  // Teardown may cancel a phase the controller never reached.
  void promise.catch(() => {});
  return { promise, resolve, reject };
}

export class SimulatedUpdates implements UpdateDriver {
  readonly trace: string[] = [];
  private checkResult = deferred<UpdateSession | null>();
  private downloadResult = deferred<void>();
  private installResult = deferred<void>();
  private restartResult = deferred<void>();
  private progress?: (progress: DownloadProgress) => void;
  private cancelled = false;
  private cancelEvents: Array<() => void> = [];
  private scenario: PreviewScenario;
  private clock: Clock;
  constructor(clock: Clock, scenario: PreviewScenario) { this.clock = clock; this.scenario = scenario; }
  check() {
    this.trace.push('check');
    if (this.trace.filter(x => x === 'check').length > 1) {
      this.checkResult = deferred(); this.downloadResult = deferred(); this.installResult = deferred(); this.restartResult = deferred();
    }
    const delay = this.scenario === 'late' ? 2300 : this.scenario === 'slow' ? 6000 : this.scenario === 'ordinary' ? 100 : 300;
    this.cancelEvents.push(this.clock.schedule(delay, () => this.checkResult.resolve(this.scenario === 'ordinary' || this.scenario === 'slow' ? null : this.session())));
    return this.checkResult.promise;
  }
  private session(): UpdateSession {
    return {
      version: '0.3.4-preview',
      download: onProgress => {
        this.trace.push('download'); this.progress = onProgress;
        if (this.scenario === 'install' || this.scenario === 'install-error' || this.scenario === 'restart-error') this.cancelEvents.push(this.clock.schedule(500, () => this.downloadComplete()));
        if (this.scenario === 'download-error') this.cancelEvents.push(this.clock.schedule(800, () => this.fail('download')));
        return this.downloadResult.promise;
      },
      install: () => {
        this.trace.push('install');
        if (this.scenario === 'install-error') this.cancelEvents.push(this.clock.schedule(700, () => this.fail('install')));
        if (this.scenario === 'restart-error') this.cancelEvents.push(this.clock.schedule(700, () => this.installComplete()));
        return this.installResult.promise;
      },
      relaunch: () => {
        this.trace.push('relaunch');
        const count = this.trace.filter(step => step === 'relaunch').length;
        if (count > 1) this.restartResult = deferred();
        if (this.scenario === 'restart-error' && count === 1) return Promise.reject(new Error('Simulated restart failure'));
        return this.restartResult.promise;
      },
      close: async () => { this.trace.push('close'); },
    };
  }
  downloadComplete() { if (!this.cancelled) { this.progress?.({ downloaded: 100, total: 100 }); this.downloadResult.resolve(); } }
  installComplete() { if (!this.cancelled) this.installResult.resolve(); }
  restartComplete() { if (!this.cancelled) this.restartResult.resolve(); }
  fail(phase: 'download' | 'install' | 'restart') {
    const error = new Error(`Simulated ${phase} failure`);
    if (phase === 'download') this.downloadResult.reject(error);
    if (phase === 'install') this.installResult.reject(error);
    if (phase === 'restart') this.restartResult.reject(error);
  }
  dispose() {
    this.cancelled = true; this.cancelEvents.forEach(cancel => cancel());
    this.checkResult.reject(new Error('Preview stopped')); this.downloadResult.reject(new Error('Preview stopped'));
    this.installResult.reject(new Error('Preview stopped')); this.restartResult.reject(new Error('Preview stopped'));
  }
}

export class BrowserPresentation implements PresentationDriver {
  mode: VisualMode = 'hidden';
  revision = 0;
  private origin = 0;
  private offset = 0;
  private frozenElapsed = 0;
  private request?: PresentationRequest;
  private completion?: () => void;
  private completionCalled = false;
  private cancelEnd?: () => void;
  private clock: Clock;
  private tuning: Readonly<StartupTuning>;
  constructor(clock: Clock, tuning = DEFAULT_TUNING) { this.clock = clock; this.tuning = tuning; }
  get elapsed() { return this.mode === 'hidden' ? this.frozenElapsed : Math.max(0, this.clock.now() - this.origin + this.offset); }
  get duration() { return this.mode === 'compact-intro' ? REDUCED_INTRO_MS : introDuration(this.tuning); }
  async present(request: PresentationRequest, onComplete: () => void) {
    if (request.revision < this.revision) return;
    if (request.revision === this.revision) {
      if (this.request?.revision === request.revision && this.request.mode === request.mode && this.request.durationMs === request.durationMs) return;
      throw new Error('Conflicting preview presentation revision');
    }
    this.cancelEnd?.(); this.revision = request.revision; this.mode = request.mode;
    this.request = request; this.origin = this.clock.now(); this.offset = 0;
    this.completion = onComplete; this.completionCalled = false; this.scheduleEnd();
  }
  async hide(revision: number) {
    if (revision < this.revision) return;
    this.frozenElapsed = this.elapsed;
    this.cancelEnd?.(); this.revision = revision; this.mode = 'hidden';
  }
  private scheduleEnd() {
    this.cancelEnd?.();
    if (this.mode !== 'cinematic' && this.mode !== 'compact-intro') return;
    this.cancelEnd = this.clock.schedule(Math.max(0, this.duration - this.elapsed), () => {
      this.frozenElapsed = this.duration; this.mode = 'hidden';
      if (!this.completionCalled) { this.completionCalled = true; this.completion?.(); }
    });
  }
  seek(elapsedMs: number) {
    if (!this.request) return;
    // Pure visual sampling, including after the intro ended. No fake update advances.
    this.mode = this.request.mode;
    this.offset = Math.max(0, elapsedMs) - (this.clock.now() - this.origin);
    this.scheduleEnd();
  }
  syncElapsed(elapsedMs: number) {
    if (this.mode === 'hidden') return;
    this.offset = Math.max(0, elapsedMs) - (this.clock.now() - this.origin); this.scheduleEnd();
  }
  setTuning(tuning: Readonly<StartupTuning>) { this.tuning = tuning; this.scheduleEnd(); }
  dispose() { this.cancelEnd?.(); this.mode = 'hidden'; }
}

/** Mirror simulated presentations into an explicitly enabled debug native surface. */
class MirroredPresentation extends BrowserPresentation {
  private native: NativePresentation;
  private latest = 0;
  constructor(clock: Clock, tuning: Readonly<StartupTuning>, native: NativePresentation) { super(clock, tuning); this.native = native; }
  override async present(request: PresentationRequest, complete: () => void) {
    this.latest = request.revision;
    await this.native.present({ ...request, revision: this.native.nextPreviewRevision() }, complete);
    if (this.latest === request.revision) await super.present(request, complete);
  }
  override async hide(revision: number) {
    if (revision < this.latest) return;
    this.latest = revision;
    const nativeHide = this.native.hide(this.native.nextPreviewRevision());
    await super.hide(revision);
    await nativeHide;
  }
  override seek(elapsed: number) { super.seek(elapsed); void this.native.control({ action: 'seek', elapsedMs: elapsed }).catch(() => {}); }
  override setTuning(tuning: Readonly<StartupTuning>) { super.setTuning(tuning); void this.native.control({ action: 'tune', tuning }).catch(() => {}); }
}

/** The preview never imports updater/process/activation APIs. */
export class StartupPreview {
  readonly clock = new VirtualClock();
  readonly updates: SimulatedUpdates;
  readonly presentation: BrowserPresentation;
  readonly controller: StartupController;
  paused = false;
  private disposed = false;
  private native?: NativePresentation;
  constructor(scenario: PreviewScenario = 'ordinary', tuning = DEFAULT_TUNING, reducedMotion = false, native?: NativePresentation) {
    const validated = validateTuning({ ...tuning });
    this.updates = new SimulatedUpdates(this.clock, scenario);
    this.native = native; native?.configure(validated, reducedMotion);
    this.presentation = native ? new MirroredPresentation(this.clock, validated, native) : new BrowserPresentation(this.clock, validated);
    this.controller = new StartupController({ clock: this.clock, updates: this.updates, presentation: this.presentation, tuning: validated, reducedMotion });
    this.controller.start();
  }
  advance(deltaMs: number) { if (!this.paused && !this.disposed) this.clock.advance(deltaMs); }
  pause() { this.paused = true; void this.native?.control({ action: 'pause' }).catch(() => {}); }
  resume() { if (!this.disposed) { this.paused = false; void this.native?.control({ action: 'resume' }).catch(() => {}); } }
  seek(elapsedMs: number) { this.pause(); this.presentation.seek(elapsedMs); }
  tune(tuning: Readonly<StartupTuning>) { this.presentation.setTuning(validateTuning({ ...tuning })); }
  dispose() {
    if (this.disposed) return;
    this.disposed = true; this.controller.dispose(); this.presentation.dispose(); this.updates.dispose(); this.clock.clear();
  }
}
