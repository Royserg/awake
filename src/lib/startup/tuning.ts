import type { StartupTuning } from './types.ts';

export const DEFAULT_TUNING: Readonly<StartupTuning> = Object.freeze({
  openingMs: 850, holdMs: 600, exitMs: 300,
  backdropEnterMs: 120, waveStartMs: 200, introCellPx: 15,
  waveOpacity: 0.25, waveEnabled: true,
  backdropCenterAlpha: 0.55, backdropEdgeAlpha: 0.30,
  idleCycleMs: 2400, idleAmplitudeRows: 0.5,
});

export const INTRO_PREPARE_MS = 2000;
export const COMPACT_PAINT_MS = 50;
export const CHECK_TIMEOUT_MS = 10000;
export const STALL_MS = 60000;
export const REDUCED_INTRO_MS = 250;
export const MAX_WAVE_CELLS = 256;
export const introDuration = (tuning: Readonly<StartupTuning>) => tuning.openingMs + tuning.holdMs + tuning.exitMs;

export function validateTuning(input: StartupTuning): Readonly<StartupTuning> {
  const tuning = { ...input };
  for (const [key, value] of Object.entries(tuning)) {
    if (key !== 'waveEnabled' && (typeof value !== 'number' || !Number.isFinite(value))) throw new Error(`${key} must be finite`);
  }
  if (typeof tuning.waveEnabled !== 'boolean') throw new Error('waveEnabled must be boolean');
  if (tuning.openingMs <= 0 || tuning.exitMs <= 0 || tuning.holdMs < 0 || introDuration(tuning) < 1500 || introDuration(tuning) > 2000) throw new Error('The complete intro must last 1500–2000 ms');
  const between = (value: number, min: number, max: number) => value >= min && value <= max;
  if (!between(tuning.backdropEnterMs, 0, tuning.openingMs)) throw new Error('Backdrop entry must finish during opening');
  if (!between(tuning.waveStartMs, 0, tuning.openingMs - 100)) throw new Error('Wave needs at least 100 ms to travel');
  if (!Number.isInteger(tuning.introCellPx) || !between(tuning.introCellPx, 8, 24)) throw new Error('Cell size must be 8–24 CSS pixels');
  if (!between(tuning.waveOpacity, 0, 0.4)) throw new Error('Wave opacity must be 0–0.4');
  if (!between(tuning.backdropCenterAlpha, 0, 0.7) || !between(tuning.backdropEdgeAlpha, 0, 0.7)) throw new Error('Backdrop opacity must be 0–0.7');
  if (!between(tuning.idleCycleMs, 1800, 4000) || !between(tuning.idleAmplitudeRows, 0, 1)) throw new Error('Idle cycle or amplitude is outside its range');
  return Object.freeze(tuning);
}
