import type { StartupFrame, StartupTuning, VisualMode } from './types.ts';
import { DEFAULT_TUNING, introDuration, MAX_WAVE_CELLS, REDUCED_INTRO_MS } from './tuning.ts';

const clamp = (value: number) => Math.max(0, Math.min(1, value));
const smootherstep = (value: number) => { const t = clamp(value); return t*t*t*(t*(t*6-15)+10); };

/** Pure elapsed-time sampling; no frame counters, native APIs, timers or DOM. */
export function sampleStartupFrame(
  elapsedMs: number, mode: VisualMode,
  tuning: Readonly<StartupTuning> = DEFAULT_TUNING,
  width = 1280, height = 800, reducedMotion = false,
): StartupFrame {
  const time = Math.max(0, Number.isFinite(elapsedMs) ? elapsedMs : 0);
  const empty: StartupFrame = { eyeProgress: 0, eyeOffsetPx: 0, opacity: 1, backdropCenter: 0, backdropEdge: 0, waveOpacity: 0, waveCells: [], complete: false, phase: 'hidden' };
  if (mode === 'hidden') return { ...empty, opacity: 0, complete: true };
  if (mode === 'compact-error') return { ...empty, phase: 'error' };
  if (mode === 'compact-wait') {
    const cycle = time % tuning.idleCycleMs;
    const lift = cycle < 120 ? smootherstep(cycle / 120) : 1 - smootherstep((cycle - 120) / 240);
    return { ...empty, phase: 'waiting', eyeOffsetPx: reducedMotion ? 0 : -Math.round(lift * tuning.idleAmplitudeRows * 15) };
  }
  if (reducedMotion || mode === 'compact-intro') return { ...empty, eyeProgress: 100, complete: time >= REDUCED_INTRO_MS, opacity: time >= REDUCED_INTRO_MS ? 0 : 1, phase: 'hold' };

  const total = introDuration(tuning), exitStart = tuning.openingMs + tuning.holdMs;
  const opacity = 1 - smootherstep((time - exitStart) / tuning.exitMs);
  const backdrop = tuning.backdropEnterMs === 0 ? 1 : smootherstep(time / tuning.backdropEnterMs);
  const phase = time < tuning.openingMs ? 'opening' : time < exitStart ? 'hold' : 'exit';
  const frame: StartupFrame = { ...empty, phase, eyeProgress: 100 * smootherstep(time / tuning.openingMs), opacity, backdropCenter: backdrop * tuning.backdropCenterAlpha, backdropEdge: backdrop * tuning.backdropEdgeAlpha, complete: time >= total };
  if (!tuning.waveEnabled || time < tuning.waveStartMs || time >= tuning.openingMs || width <= 0 || height <= 0) return frame;

  const travel = clamp((time - tuning.waveStartMs) / (tuning.openingMs - tuning.waveStartMs));
  const cell = tuning.introCellPx, reach = Math.hypot(width / 2, height / 2) + cell;
  const start = cell * 5, radius = start + (reach - start) * (1 - (1 - travel)**2);
  const cells: Array<{ x: number; y: number }> = [], seen = new Set<string>();
  const count = Math.min(MAX_WAVE_CELLS, Math.max(16, Math.ceil(2 * Math.PI * radius / cell)));
  for (let i = 0; i < count; i++) {
    const angle = i / count * Math.PI * 2;
    const x = Math.round(Math.cos(angle) * radius / cell) * cell + width / 2 - cell / 2;
    const y = Math.round(Math.sin(angle) * radius / cell) * cell + height / 2 - cell / 2;
    const key = `${x},${y}`;
    if (x < -cell || y < -cell || x > width || y > height || seen.has(key)) continue;
    seen.add(key); cells.push({ x, y });
  }
  return { ...frame, waveCells: cells, waveOpacity: tuning.waveOpacity * smootherstep(travel / 0.12) * (1 - smootherstep((travel - 0.65) / 0.35)) };
}
