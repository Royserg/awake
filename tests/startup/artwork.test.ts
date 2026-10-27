import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { PixelAnimation, sampleEyeArtwork } from '../../src/routes/eye-splashscreen.ts';
import { PixelAnimation as SharedPixelAnimation, sampleEyeArtwork as sharedSampler } from '../../src/lib/startup/pixel-eye.ts';

const fixtures = JSON.parse(readFileSync(new URL('./fixtures/eye-poses.json', import.meta.url), 'utf8'));
const normalized = (value: unknown) => JSON.parse(JSON.stringify(value));

test('legacy exports forward to the single shared renderer and sampler', () => {
  assert.equal(PixelAnimation, SharedPixelAnimation);
  assert.equal(sampleEyeArtwork, sharedSampler);
});

test('legacy and startup endpoints preserve the pre-change pixel artwork', () => {
  for (const motion of ['legacy', 'startup'] as const) {
    assert.deepEqual(normalized(sampleEyeArtwork(0, { motion })), fixtures.closed);
    assert.deepEqual(normalized(sampleEyeArtwork(100, { motion })), fixtures.open);
  }
});
test('transparent rendering removes only the named canvas background', () => {
  assert.deepEqual(normalized(sampleEyeArtwork(100, { background: 'transparent' })), fixtures.open.slice(1));
  assert.equal(sampleEyeArtwork(100).length, 3);
});
test('the pupil fades in instead of appearing fully opaque at its first keyframe', () => {
  const pupil = (progress: number, motion: 'startup' | 'legacy') => sampleEyeArtwork(progress, { motion }).at(-1)!.objectSnapshots;
  assert.equal(pupil(67, 'startup').length, 0);
  assert.equal(pupil(68, 'startup')[0].transform.opacity, 0);
  assert.ok(pupil(73, 'startup')[0].transform.opacity > 0 && pupil(73, 'startup')[0].transform.opacity < 1);
  assert.equal(pupil(68, 'legacy')[0].transform.opacity, 1);
});
test('all eased samples remain finite and opacity stays bounded', () => {
  for (let progress = 0; progress <= 100; progress += 0.5) {
    for (const layer of sampleEyeArtwork(progress, { motion: 'startup' })) {
      for (const transform of [layer.transform, ...layer.objectSnapshots.map(object => object.transform)]) {
        assert.ok(Object.values(transform).every(Number.isFinite));
        assert.ok(transform.opacity >= 0 && transform.opacity <= 1);
      }
    }
  }
});
