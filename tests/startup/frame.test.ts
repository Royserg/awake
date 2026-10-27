import { test } from 'node:test';
import assert from 'node:assert/strict';
import { sampleStartupFrame } from '../../src/lib/startup/frame.ts';
import { DEFAULT_TUNING, introDuration, validateTuning } from '../../src/lib/startup/tuning.ts';

test('opening, hold, and exit have exact elapsed-time endpoints', () => {
  assert.equal(introDuration(DEFAULT_TUNING), 1750);
  assert.equal(sampleStartupFrame(0, 'cinematic').eyeProgress, 0);
  assert.equal(sampleStartupFrame(850, 'cinematic').eyeProgress, 100);
  assert.equal(sampleStartupFrame(1200, 'cinematic').phase, 'hold');
  assert.equal(sampleStartupFrame(1450, 'cinematic').opacity, 1);
  const end = sampleStartupFrame(1750, 'cinematic');
  assert.equal(end.opacity, 0); assert.equal(end.complete, true);
  let previous = 0;
  for (let t = 0; t <= 850; t++) { const frame = sampleStartupFrame(t, 'cinematic'); assert.ok(frame.eyeProgress >= previous); previous = frame.eyeProgress; }
});
test('wave is bounded, deduplicated, and reaches window edges on varied viewports', () => {
  for (const [width, height] of [[300, 300], [1920, 1080], [3840, 2160], [3440, 1440]]) {
    let reachedEdge = false;
    for (let t = 200; t < 850; t += 5) {
      const frame = sampleStartupFrame(t, 'cinematic', DEFAULT_TUNING, width, height);
      assert.ok(frame.waveCells.length <= 256);
      assert.equal(new Set(frame.waveCells.map(p => `${p.x},${p.y}`)).size, frame.waveCells.length);
      reachedEdge ||= frame.waveCells.some(p => p.x <= 15 || p.y <= 15 || p.x >= width - 30 || p.y >= height - 30);
    }
    assert.ok(reachedEdge, `${width}×${height} wave must reach the edge`);
  }
  assert.equal(sampleStartupFrame(100, 'cinematic').waveCells.length, 0);
  assert.equal(sampleStartupFrame(850, 'cinematic').waveCells.length, 0);
});
test('waiting never opens the eye and reduced motion has no travel or dimming', () => {
  for (let t = 0; t < 10000; t += 25) {
    const frame = sampleStartupFrame(t, 'compact-wait');
    assert.equal(frame.eyeProgress, 0); assert.equal(frame.complete, false);
    assert.ok(Math.abs(frame.eyeOffsetPx) <= 15);
    assert.equal(frame.waveCells.length, 0); assert.equal(frame.backdropCenter, 0);
    assert.equal(sampleStartupFrame(t, 'compact-wait', DEFAULT_TUNING, 300, 300, true).eyeOffsetPx, 0);
  }
  const reduced = sampleStartupFrame(100, 'compact-intro', DEFAULT_TUNING, 300, 300, true);
  assert.equal(reduced.eyeProgress, 100); assert.equal(reduced.waveCells.length, 0); assert.equal(reduced.backdropCenter, 0);
  assert.equal(sampleStartupFrame(250, 'compact-intro').complete, true);
});
test('invalid tuning is rejected and default values remain immutable', () => {
  assert.throws(() => validateTuning({ ...DEFAULT_TUNING, openingMs: NaN }));
  assert.throws(() => validateTuning({ ...DEFAULT_TUNING, holdMs: 4000 }));
  assert.throws(() => validateTuning({ ...DEFAULT_TUNING, introCellPx: 15.5 }));
  assert.throws(() => validateTuning({ ...DEFAULT_TUNING, waveStartMs: 850 }));
  assert.throws(() => validateTuning({ ...DEFAULT_TUNING, idleAmplitudeRows: 2 }));
  assert.ok(Object.isFrozen(DEFAULT_TUNING));
});
