import { test } from 'node:test';
import assert from 'node:assert/strict';
import { deferred, flush, harness } from './helpers.ts';

test('checking starts once before presentation; ordinary hiding does not wait for a slow check', async () => {
  const h = harness(); h.controller.start(); h.controller.start();
  assert.deepEqual(h.trace, ['check']);
  h.clock.advance(1750); await flush();
  assert.equal(h.controller.snapshot.mode, 'hidden');
  assert.equal(h.controller.snapshot.updatePhase, 'checking');
  h.checks[0].resolve(null); await flush();
  assert.equal(h.controller.snapshot.updatePhase, 'no-update'); h.controller.dispose();
});
test('visual setup failure cannot bypass checking or a found update', async () => {
  const h = harness(); h.presentation.fail = true; h.controller.start(); await flush();
  assert.equal(h.trace[0], 'check'); assert.equal(h.controller.snapshot.mode, 'hidden');
  h.checks[0].resolve(h.session); await flush();
  assert.ok(h.trace.includes('download')); assert.ok(h.errors.length >= 1);
  h.download.reject(new Error('test finished')); await flush(); h.controller.dispose();
});
test('an early update supersedes the intro and stale completion cannot hide waiting', async () => {
  const h = harness(); h.controller.start(); h.clock.advance(300);
  const oldCompletion = h.presentation.completions[0];
  h.checks[0].resolve(h.session); await flush();
  assert.equal(h.controller.snapshot.mode, 'compact-wait');
  assert.deepEqual(h.trace, ['check', 'download']);
  const hides = h.presentation.hides.length;
  oldCompletion(); assert.equal(h.presentation.hides.length, hides);
  h.download.resolve(); await flush();
  assert.deepEqual(h.trace, ['check', 'download', 'install']);
  h.install.resolve(); await flush(); assert.equal(h.trace.at(-1), 'relaunch');
  h.restart.resolve(); await flush(); assert.equal(h.trace.at(-1), 'close'); h.controller.dispose();
});
test('a late update opens compact waiting without replaying the intro', async () => {
  const h = harness(); h.controller.start(); h.clock.advance(2300); await flush();
  h.checks[0].resolve(h.session); await flush();
  assert.deepEqual(h.presentation.requests.map(r => r.mode), ['cinematic', 'compact-wait']);
  h.download.reject(new Error('test finished')); await flush(); h.controller.dispose();
});
test('hiding retains the update chain and a later failure cannot reopen the wait', async () => {
  const h = harness(); h.controller.start(); h.checks[0].resolve(h.session); await flush();
  h.controller.hideWait(); const shows = h.presentation.requests.length;
  h.download.resolve(); await flush(); assert.ok(h.trace.includes('install'));
  h.install.reject(new Error('Installation failed')); await flush();
  assert.equal(h.controller.snapshot.mode, 'hidden'); assert.equal(h.presentation.requests.length, shows);
  assert.equal(h.controller.snapshot.failurePhase, 'installing');
  assert.equal(h.trace.filter(x => x === 'close').length, 1); h.controller.dispose();
});
test('download retry waits for settled cleanup and duplicate actions do not duplicate checks', async () => {
  const h = harness(); h.controller.start(); h.checks[0].resolve(h.session); await flush();
  h.download.reject(new Error('Download failed')); await flush();
  assert.equal(h.controller.snapshot.failurePhase, 'downloading'); assert.equal(h.trace.at(-1), 'close');
  h.controller.retryDownload(); h.controller.retryDownload();
  assert.equal(h.trace.filter(x => x === 'check').length, 2);
  h.checks[1].resolve(null); await flush(); h.controller.dispose();
});
test('restart retry invokes only relaunch after installation already finished', async () => {
  const h = harness(); h.controller.start(); h.checks[0].resolve(h.session); await flush();
  h.download.resolve(); await flush(); h.install.resolve(); await flush();
  h.restart.reject(new Error('Restart failed')); await flush();
  assert.equal(h.controller.snapshot.failurePhase, 'restarting');
  await h.controller.retryRestart(); await flush();
  assert.equal(h.trace.filter(x => x === 'relaunch').length, 2);
  assert.equal(h.trace.filter(x => x === 'download').length, 1);
  assert.equal(h.trace.filter(x => x === 'install').length, 1); h.controller.dispose();
});
test('timed-out late check resources close without starting a download', async () => {
  const h = harness(); h.controller.start(); h.clock.advance(10000); await flush();
  h.checks[0].resolve(h.session); await flush();
  assert.deepEqual(h.trace, ['check', 'close']); assert.equal(h.controller.snapshot.mode, 'hidden'); h.controller.dispose();
});
test('installation waits for the phase DOM flush without depending on hidden animation frames', async () => {
  const painted = deferred<void>();
  const h = harness({ beforeInstall: () => painted.promise });
  h.controller.start(); h.checks[0].resolve(h.session); await flush();
  h.download.resolve(); await flush();
  assert.equal(h.controller.snapshot.updatePhase, 'installing'); assert.ok(!h.trace.includes('install'));
  painted.resolve(); await flush(); assert.ok(h.trace.includes('install'));
  h.install.reject(new Error('test finished')); await flush(); h.controller.dispose();
});
test('compatibility splash never rechecks or displaces an active update', async () => {
  const h = harness(); h.controller.start(); h.controller.showStaticIntro();
  assert.equal(h.controller.snapshot.mode, 'compact-intro'); assert.equal(h.trace.filter(x => x === 'check').length, 1);
  h.checks[0].resolve(h.session); await flush();
  h.controller.showStaticIntro(); assert.equal(h.controller.snapshot.mode, 'compact-wait');
  h.download.reject(new Error('test finished')); await flush(); h.controller.dispose();
});
