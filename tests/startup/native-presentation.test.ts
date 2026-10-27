import { test } from 'node:test';
import assert from 'node:assert/strict';
import { NativePresentation, type NativePort, type NativeReceipt } from '../../src/lib/startup/native-presentation.ts';
import { VirtualClock } from '../../src/lib/startup/preview-driver.ts';
import { deferred, flush } from './helpers.ts';

class Port implements NativePort {
  calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  listeners = new Map<string, (payload: unknown) => void>();
  start = deferred<NativeReceipt>();
  failContext = false;
  async listen<T>(event: string, callback: (payload: T) => void) {
    this.listeners.set(event, callback as (payload: unknown) => void);
    return () => { this.listeners.delete(event); };
  }
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    this.calls.push({ command, args });
    if (command.endsWith('_context')) { if (this.failContext) throw new Error('No native context'); return 10 as T; }
    if (command === 'start_intro' || command === 'preview_present') return this.start.promise as Promise<T>;
    if (command === 'show_compact') return receipt(Number(args?.revision), 'active', `compact-${args?.revision}`) as T;
    return undefined as T;
  }
  emit(value: NativeReceipt) { this.listeners.get('startup-presentation')?.(value); }
}
function receipt(revision: number, status: NativeReceipt['status'], runId = 'intro'): NativeReceipt {
  return { revision, status, runId, compactToken: runId.startsWith('compact') ? runId : null, error: null };
}

test('native ready before command response is monotonic and duplicate active cannot reset the visual', async () => {
  const port = new Port(), clock = new VirtualClock(), visible: string[] = [];
  const driver = new NativePresentation(port, { scope: 'production', clock, visible: mode => visible.push(mode) });
  let completed = 0;
  const showing = driver.present({ mode: 'cinematic', revision: 1 }, () => completed++);
  await flush();
  assert.equal(port.calls.at(-1)?.command, 'start_intro');
  port.emit(receipt(11, 'active')); port.emit(receipt(11, 'active'));
  port.start.resolve(receipt(11, 'preparing')); await showing;
  assert.deepEqual(visible, ['cinematic']);
  port.emit(receipt(11, 'ended')); port.emit(receipt(11, 'ended'));
  assert.equal(completed, 1); await driver.dispose(); assert.equal(port.listeners.size, 0);
});
test('hide barriers and token-only late cleanup cannot hide a newer compact wait', async () => {
  const port = new Port(), driver = new NativePresentation(port, { scope: 'production', clock: new VirtualClock() });
  let staleCompletion = 0;
  const intro = driver.present({ mode: 'cinematic', revision: 1 }, () => staleCompletion++); await flush();
  await driver.hide(2);
  assert.ok(port.calls.some(c => c.command === 'finish_intro' && c.args?.runId === null && c.args.revision === 12));
  await driver.present({ mode: 'compact-wait', revision: 3 }, () => {});
  port.start.resolve(receipt(11, 'active')); await intro;
  const cleanup = port.calls.at(-1)!;
  assert.equal(cleanup.command, 'finish_intro'); assert.equal(cleanup.args?.runId, 'intro'); assert.equal(cleanup.args?.revision, null);
  port.emit(receipt(11, 'ended')); assert.equal(staleCompletion, 0); await driver.dispose();
});
test('a native initialization failure does not create an uncleanable fallback window', async () => {
  const port = new Port(); port.failContext = true;
  const driver = new NativePresentation(port, { scope: 'production', clock: new VirtualClock() });
  await assert.rejects(driver.present({ mode: 'cinematic', revision: 1 }, () => {}));
  assert.ok(!port.calls.some(c => c.command === 'show_compact' || c.command === 'start_intro'));
  await assert.rejects(driver.dispose()); assert.equal(port.listeners.size, 0);
});
test('a failed cinematic uses a short compact fallback and respects newer revisions', async () => {
  const port = new Port(), clock = new VirtualClock(), modes: string[] = [];
  const driver = new NativePresentation(port, { scope: 'production', clock, prepare: async mode => { modes.push(mode); } });
  let completed = 0;
  const showing = driver.present({ mode: 'cinematic', revision: 1 }, () => completed++); await flush();
  port.start.reject(new Error('Transparent windows unavailable')); await showing;
  assert.deepEqual(modes, ['cinematic', 'compact-intro']);
  assert.equal(port.calls.at(-1)?.args?.kind, 'intro');
  clock.advance(249); assert.equal(completed, 0); clock.advance(1); assert.equal(completed, 1);
  await driver.present({ mode: 'compact-wait', revision: 2 }, () => {});
  clock.advance(1000); assert.equal(completed, 1); await driver.dispose();
});
test('preview controls queue until readiness and use only the preview namespace', async () => {
  const port = new Port(), driver = new NativePresentation(port, { scope: 'preview', clock: new VirtualClock() });
  const showing = driver.present({ mode: 'cinematic', revision: driver.nextPreviewRevision() }, () => {}); await flush();
  await driver.control({ action: 'pause' });
  assert.ok(!port.calls.some(c => c.command === 'preview_control'));
  port.start.resolve(receipt(11, 'preparing')); await flush();
  await driver.control({ action: 'seek', elapsedMs: 700 });
  port.emit(receipt(11, 'active')); await showing; await flush();
  assert.deepEqual(port.calls.filter(c => c.command === 'preview_control').map(c => (c.args?.control as { action: string }).action), ['pause', 'seek']);
  port.emit(receipt(11, 'ended'));
  const sent = port.calls.length; await driver.control({ action: 'resume' });
  assert.equal(port.calls.length, sent, 'terminal runs cannot queue or forward controls');
  await driver.hide(driver.nextPreviewRevision()); await driver.dispose();
  assert.ok(port.calls.every(c => c.command.startsWith('preview_')));
});

test('a hide revision is a tombstone and cannot resurrect an ended native run', async () => {
  const port = new Port(), driver = new NativePresentation(port, { scope: 'production', clock: new VirtualClock() });
  await driver.hide(2);
  const sent = port.calls.length;
  await driver.hide(2);
  await assert.rejects(driver.present({ mode: 'compact-wait', revision: 2 }, () => {}), /revision_conflict/);
  assert.equal(port.calls.length, sent);
  await driver.present({ mode: 'compact-wait', revision: 3 }, () => {});
  assert.equal(port.calls.at(-1)?.command, 'show_compact');
  await driver.dispose();
});
