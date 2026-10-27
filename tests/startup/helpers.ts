import type { Clock, PresentationDriver, PresentationRequest, UpdateDriver, UpdateSession } from '../../src/lib/startup/types.ts';
import { VirtualClock } from '../../src/lib/startup/preview-driver.ts';
import { StartupController, type ControllerOptions } from '../../src/lib/startup/controller.ts';

export function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void, reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  void promise.catch(() => {});
  return { promise, resolve, reject };
}
export async function flush() { for (let i = 0; i < 20; i++) await Promise.resolve(); }

export class RecordingPresentation implements PresentationDriver {
  readonly requests: PresentationRequest[] = [];
  readonly completions: Array<() => void> = [];
  readonly hides: number[] = [];
  fail = false;
  private clock: Clock;
  private cancel?: () => void;
  constructor(clock: Clock) { this.clock = clock; }
  async present(request: PresentationRequest, complete: () => void) {
    this.requests.push(request); this.completions.push(complete); this.cancel?.();
    if (this.fail) throw new Error('Presentation unavailable');
    if (request.durationMs) this.cancel = this.clock.schedule(request.durationMs, complete);
  }
  async hide(revision: number) { this.hides.push(revision); this.cancel?.(); }
}

export function harness(overrides: Partial<ControllerOptions> = {}) {
  const clock = new VirtualClock();
  const trace: string[] = [];
  const checks = [deferred<UpdateSession | null>(), deferred<UpdateSession | null>()];
  const download = deferred<void>(), install = deferred<void>(), restart = deferred<void>();
  let checkIndex = 0;
  const updates: UpdateDriver = { check: () => { trace.push('check'); return checks[checkIndex++].promise; } };
  const session: UpdateSession = {
    version: 'test', download: async () => { trace.push('download'); await download.promise; },
    install: async () => { trace.push('install'); await install.promise; },
    relaunch: async () => { trace.push('relaunch'); await restart.promise; },
    close: async () => { trace.push('close'); },
  };
  const presentation = new RecordingPresentation(clock);
  const errors: unknown[] = [];
  const controller = new StartupController({ clock, updates, presentation, onVisualError: error => errors.push(error), ...overrides });
  return { clock, trace, checks, download, install, restart, session, presentation, errors, controller };
}
