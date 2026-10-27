export interface StartupTuning {
  openingMs: number;
  holdMs: number;
  exitMs: number;
  backdropEnterMs: number;
  waveStartMs: number;
  introCellPx: number;
  waveOpacity: number;
  waveEnabled: boolean;
  backdropCenterAlpha: number;
  backdropEdgeAlpha: number;
  idleCycleMs: number;
  idleAmplitudeRows: number;
}

export type VisualMode = 'cinematic' | 'compact-intro' | 'compact-wait' | 'compact-error' | 'hidden';
export type UpdatePhase = 'checking' | 'no-update' | 'downloading' | 'installing' | 'restarting' | 'failed';
export type FailurePhase = 'checking' | 'downloading' | 'installing' | 'restarting';

export interface StartupFrame {
  eyeProgress: number;
  eyeOffsetPx: number;
  opacity: number;
  backdropCenter: number;
  backdropEdge: number;
  waveOpacity: number;
  waveCells: ReadonlyArray<{ x: number; y: number }>;
  complete: boolean;
  phase: 'opening' | 'hold' | 'exit' | 'waiting' | 'error' | 'hidden';
}

export interface Clock {
  now(): number;
  schedule(delayMs: number, callback: () => void): () => void;
}

export interface DownloadProgress { downloaded: number; total?: number }
export interface UpdateSession {
  version: string;
  download(onProgress: (progress: DownloadProgress) => void): Promise<void>;
  install(): Promise<void>;
  relaunch(): Promise<void>;
  close(): Promise<void>;
}
export interface UpdateDriver { check(): Promise<UpdateSession | null> }

export interface PresentationRequest {
  revision: number;
  mode: Exclude<VisualMode, 'hidden'>;
  durationMs?: number;
}
export interface PresentationDriver {
  present(request: PresentationRequest, onComplete: () => void): Promise<void>;
  hide(revision: number): Promise<void>;
}

export interface StartupSnapshot {
  mode: VisualMode;
  updatePhase: UpdatePhase;
  revision: number;
  version: string | null;
  progress: Readonly<DownloadProgress> | null;
  failurePhase: FailurePhase | null;
  error: string | null;
  hiddenByUser: boolean;
  stalled: boolean;
  busy: boolean;
}
