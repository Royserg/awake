/** Compatibility entry point; canonical artwork and rendering live in the shared library. */
export { PixelAnimation, sampleEyeArtwork } from '../lib/startup/pixel-eye.ts';
export type {
  EyeArtworkLayer,
  PixelAnimationController,
  PixelAnimationOptions,
  PixelLayerData,
  PixelObjectSnapshot,
  PixelTransform,
} from '../lib/startup/pixel-eye.ts';
