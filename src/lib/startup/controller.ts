import type { Clock, FailurePhase, PresentationDriver, StartupSnapshot, StartupTuning, UpdateDriver, UpdateSession } from './types.ts';
import { CHECK_TIMEOUT_MS, COMPACT_PAINT_MS, DEFAULT_TUNING, INTRO_PREPARE_MS, introDuration, REDUCED_INTRO_MS, STALL_MS } from './tuning.ts';

export interface ControllerOptions {
  clock: Clock;
  updates: UpdateDriver;
  presentation: PresentationDriver;
  tuning?: Readonly<StartupTuning>;
  reducedMotion?: boolean;
  onVisualError?: (error: unknown) => void;
  beforeInstall?: () => Promise<void>;
}

/** Launch/update policy, independent of Svelte, native plugins and animation frames. */
export class StartupController {
  private options: ControllerOptions;
  private state: Readonly<StartupSnapshot> = Object.freeze({ mode: 'hidden', updatePhase: 'checking', revision: 0, version: null, progress: null, failurePhase: null, error: null, hiddenByUser: false, stalled: false, busy: false });
  private listeners = new Set<(state: Readonly<StartupSnapshot>) => void>();
  private started = false;
  private disposed = false;
  private attempt = 0;
  private activeAttempt = false;
  private cancelCheck?: () => void;
  private cancelPreparation?: () => void;
  private cancelStall?: () => void;
  private restartAction?: () => Promise<void>;

  constructor(options: ControllerOptions) { this.options = options; }
  get snapshot() { return this.state; }
  subscribe(listener: (state: Readonly<StartupSnapshot>) => void): () => void {
    this.listeners.add(listener); listener(this.state);
    return () => this.listeners.delete(listener);
  }
  private set(patch: Partial<StartupSnapshot>) {
    this.state = Object.freeze({ ...this.state, ...patch });
    if (!this.disposed) for (const listener of this.listeners) listener(this.state);
  }

  start() {
    if (this.started || this.disposed) return;
    this.started = true;
    // Start checking before any visual preparation or acknowledgment.
    this.checkForUpdate();
    const mode = this.options.reducedMotion ? 'compact-intro' : 'cinematic';
    this.showIntroduction(mode);
  }

  /** Compatibility splash requests never interrupt a real update or recheck. */
  showStaticIntro() {
    if (this.disposed || ['downloading', 'installing', 'restarting'].includes(this.state.updatePhase)) return;
    this.showIntroduction('compact-intro');
  }

  private showIntroduction(mode: 'cinematic' | 'compact-intro') {
    this.cancelPreparation?.();
    const revision = this.state.revision + 1;
    this.set({ mode, revision });
    const durationMs = mode === 'compact-intro' ? REDUCED_INTRO_MS : introDuration(this.options.tuning ?? DEFAULT_TUNING);
    this.cancelPreparation = this.options.clock.schedule(INTRO_PREPARE_MS, () => {
      if (this.state.revision === revision) this.stopPresentation();
    });
    void this.options.presentation.present({ mode, revision, durationMs }, () => {
      if (this.state.revision === revision) this.stopPresentation();
    }).then(() => {
      if (this.state.revision === revision) this.cancelPreparation?.();
    }).catch(error => {
      this.options.onVisualError?.(error);
      if (this.state.revision === revision) this.stopPresentation();
    });
  }

  private checkForUpdate() {
    const attempt = ++this.attempt;
    this.activeAttempt = true;
    this.set({ busy: true });
    let finished = false;
    this.cancelCheck?.();
    this.cancelCheck = this.options.clock.schedule(CHECK_TIMEOUT_MS, () => {
      if (finished || attempt !== this.attempt) return;
      finished = true; this.activeAttempt = false;
      this.set({ updatePhase: 'failed', failurePhase: 'checking', error: 'Update check timed out', busy: false });
    });
    let checking: Promise<UpdateSession | null>;
    try { checking = this.options.updates.check(); }
    catch (error) { checking = Promise.reject(error); }
    void checking.then(async session => {
      if (finished || attempt !== this.attempt) { await session?.close(); return; }
      finished = true; this.cancelCheck?.();
      if (!session) { this.activeAttempt = false; this.set({ updatePhase: 'no-update', failurePhase: null, error: null, busy: false }); return; }
      await this.runUpdate(session, attempt);
    }).catch(error => {
      if (finished || attempt !== this.attempt) return;
      finished = true; this.cancelCheck?.(); this.activeAttempt = false;
      this.set({ updatePhase: 'failed', failurePhase: 'checking', error: String(error), busy: false });
    });
  }

  private async showCompact(mode: 'compact-wait' | 'compact-error') {
    this.cancelPreparation?.();
    if (this.disposed || this.state.hiddenByUser) return;
    const revision = this.state.revision + 1;
    this.set({ mode, revision });
    let cancel = () => {};
    const bounded = new Promise<void>(resolve => { cancel = this.options.clock.schedule(COMPACT_PAINT_MS, resolve); });
    const shown = this.options.presentation.present({ mode, revision }, () => {}).catch(error => { this.options.onVisualError?.(error); });
    // A decorative handoff cannot leave download waiting for a hidden frame.
    await Promise.race([shown, bounded]); cancel();
  }

  private armStall(attempt: number) {
    this.cancelStall?.();
    this.cancelStall = this.options.clock.schedule(STALL_MS, () => {
      if (attempt === this.attempt && this.state.updatePhase === 'downloading') this.set({ stalled: true });
    });
  }

  private async runUpdate(session: UpdateSession, attempt: number) {
    let phase: FailurePhase = 'downloading';
    this.restartAction = () => session.relaunch();
    this.set({ updatePhase: 'downloading', version: session.version, progress: null, error: null, failurePhase: null, stalled: false });
    await this.showCompact('compact-wait');
    try {
      this.armStall(attempt);
      await session.download(progress => {
        if (attempt !== this.attempt || this.state.updatePhase !== 'downloading') return;
        this.set({ progress: Object.freeze({ ...progress }), stalled: false }); this.armStall(attempt);
      });
      this.cancelStall?.();
      phase = 'installing'; this.set({ updatePhase: phase, stalled: false });
      try { await this.options.beforeInstall?.(); } catch (error) { this.options.onVisualError?.(error); }
      await session.install();
      phase = 'restarting'; this.set({ updatePhase: phase });
      await session.relaunch();
      this.stopPresentation();
    } catch (error) {
      this.cancelStall?.();
      this.set({ updatePhase: 'failed', failurePhase: phase, error: error instanceof Error ? error.message : String(error), stalled: false });
      await this.showCompact('compact-error');
    } finally {
      // This runs after an attempt settles, never underneath an in-flight install.
      try { await session.close(); }
      catch (error) { this.options.onVisualError?.(error); }
      this.activeAttempt = false; this.set({ busy: false });
    }
  }

  hideWait() {
    this.set({ hiddenByUser: true }); this.stopPresentation();
  }
  retryDownload() {
    if (this.activeAttempt || this.disposed || this.state.failurePhase !== 'downloading') return;
    this.set({ updatePhase: 'checking', failurePhase: null, error: null, hiddenByUser: false, progress: null });
    this.checkForUpdate();
  }
  async retryRestart() {
    if (this.activeAttempt || this.disposed || this.state.failurePhase !== 'restarting' || !this.restartAction) return;
    this.activeAttempt = true;
    this.set({ updatePhase: 'restarting', error: null, failurePhase: null, hiddenByUser: false, busy: true });
    await this.showCompact('compact-wait');
    try { await this.restartAction(); this.stopPresentation(); }
    catch (error) {
      this.set({ updatePhase: 'failed', failurePhase: 'restarting', error: String(error) });
      await this.showCompact('compact-error');
    } finally { this.activeAttempt = false; this.set({ busy: false }); }
  }
  stopPresentation() {
    this.cancelPreparation?.();
    const revision = this.state.revision + 1;
    this.set({ mode: 'hidden', revision });
    void this.options.presentation.hide(revision).catch(error => this.options.onVisualError?.(error));
  }
  dispose() {
    if (this.disposed) return;
    this.stopPresentation(); this.disposed = true; this.listeners.clear();
    this.cancelCheck?.(); this.cancelStall?.();
  }
}
