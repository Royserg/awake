import { test } from 'node:test';
import assert from 'node:assert/strict';
import { BrowserPresentation, StartupPreview, PREVIEW_SCENARIOS, VirtualClock } from '../../src/lib/startup/preview-driver.ts';
import type { PreviewScenario } from '../../src/lib/startup/preview-driver.ts';
import { flush } from './helpers.ts';

async function advance(preview: StartupPreview, ms: number) {
  for (let t = 0; t < ms; t += 20) { preview.advance(Math.min(20, ms - t)); await flush(); }
}
test('virtual clock runs stable ordered timers and cancellation', () => {
  const clock = new VirtualClock(), calls: string[] = [];
  const cancel = clock.schedule(10, () => calls.push('cancelled')); cancel();
  clock.schedule(20, () => calls.push('a')); clock.schedule(20, () => calls.push('b'));
  clock.advance(20); assert.deepEqual(calls, ['a', 'b']); assert.equal(clock.pendingCount, 0);
});
test('pause/resume/seek preserve visual time and seeking dispatches no update events', async () => {
  const preview = new StartupPreview('early'); await advance(preview, 100);
  preview.pause(); const before = preview.presentation.elapsed; preview.advance(1000);
  assert.equal(preview.presentation.elapsed, before);
  const trace = [...preview.updates.trace]; preview.seek(800);
  assert.equal(preview.presentation.elapsed, 800); assert.deepEqual(preview.updates.trace, trace);
  preview.resume(); await advance(preview, 20); assert.equal(preview.presentation.elapsed, 820);
  preview.dispose(); assert.equal(preview.clock.pendingCount, 0);
});
test('all simulated scenarios execute without native IPC and dispose pending work', async () => {
  let nativeCalls = 0;
  const original = Object.getOwnPropertyDescriptor(globalThis, 'window');
  Object.defineProperty(globalThis, 'window', { configurable: true, value: { __TAURI_INTERNALS__: { invoke: () => { nativeCalls++; throw new Error('Real IPC forbidden in preview'); } } } });
  try {
    for (const scenario of Object.keys(PREVIEW_SCENARIOS) as PreviewScenario[]) {
      const preview = new StartupPreview(scenario); await advance(preview, 2600);
      if (scenario === 'early' || scenario === 'late' || scenario === 'download') assert.equal(preview.controller.snapshot.updatePhase, 'downloading');
      if (scenario === 'install') assert.equal(preview.controller.snapshot.updatePhase, 'installing');
      if (scenario.endsWith('-error')) assert.equal(preview.controller.snapshot.updatePhase, 'failed');
      preview.dispose(); await flush(); assert.equal(preview.clock.pendingCount, 0);
      assert.equal(preview.presentation.mode, 'hidden');
    }
    assert.equal(nativeCalls, 0);
  } finally {
    if (original) Object.defineProperty(globalThis, 'window', original); else Reflect.deleteProperty(globalThis, 'window');
  }
});
test('installation requires download resolution, and simulated restart is a new launch', async () => {
  const old = new StartupPreview('download'); await advance(old, 400);
  assert.deepEqual(old.updates.trace, ['check', 'download']);
  old.updates.downloadComplete(); await flush(); assert.equal(old.controller.snapshot.updatePhase, 'installing');
  old.updates.installComplete(); await flush(); assert.equal(old.controller.snapshot.updatePhase, 'restarting');
  old.dispose(); await flush();
  const fresh = new StartupPreview('ordinary');
  assert.equal(fresh.presentation.mode, 'cinematic'); assert.equal(fresh.clock.now(), 0);
  await advance(fresh, 1750); assert.equal(fresh.presentation.mode, 'hidden'); fresh.dispose();
});
test('completed elapsed time freezes and a long frame gap reaches the current pose', async () => {
  const preview = new StartupPreview(); preview.advance(2200); await flush();
  assert.equal(preview.presentation.mode, 'hidden'); assert.equal(preview.presentation.elapsed, 1750);
  preview.advance(10000); assert.equal(preview.presentation.elapsed, 1750);
  preview.seek(700); assert.equal(preview.presentation.elapsed, 700);
  preview.resume(); preview.advance(1050); assert.equal(preview.presentation.mode, 'hidden');
  preview.dispose();
});
test('duplicate preview requests preserve the deadline and ended runs cannot be resurrected', async () => {
  const clock = new VirtualClock(), driver = new BrowserPresentation(clock);
  const request = { mode: 'cinematic', revision: 1, durationMs: 1750 } as const;
  let completed = 0;
  await driver.present(request, () => completed++); clock.advance(500);
  await driver.present(request, () => completed++);
  assert.equal(driver.elapsed, 500);
  await assert.rejects(driver.present({ ...request, mode: 'compact-wait' }, () => {}));
  clock.advance(1250); assert.equal(completed, 1); assert.equal(driver.mode, 'hidden');
  await driver.present(request, () => completed++); assert.equal(driver.mode, 'hidden');
  clock.advance(5000); assert.equal(completed, 1); driver.dispose();
});
test('reduced-motion preview is static and restart recovery never repeats install', async () => {
  const reduced = new StartupPreview('ordinary', undefined, true);
  assert.equal(reduced.presentation.mode, 'compact-intro');
  await advance(reduced, 260); assert.equal(reduced.presentation.mode, 'hidden'); reduced.dispose();
  const preview = new StartupPreview('restart-error'); await advance(preview, 1800);
  assert.equal(preview.controller.snapshot.failurePhase, 'restarting');
  void preview.controller.retryRestart(); await flush();
  assert.equal(preview.controller.snapshot.updatePhase, 'restarting');
  assert.equal(preview.updates.trace.filter(step => step === 'relaunch').length, 2);
  assert.equal(preview.updates.trace.filter(step => step === 'install').length, 1);
  preview.updates.restartComplete(); await flush(); preview.dispose();
});
