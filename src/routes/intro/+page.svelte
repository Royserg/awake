<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import StartupVisual from '../../lib/startup/StartupVisual.svelte';
  import { DEFAULT_TUNING, introDuration, REDUCED_INTRO_MS, validateTuning } from '../../lib/startup/tuning.ts';
  import type { StartupTuning, VisualMode } from '../../lib/startup/types.ts';

  interface Config { runId: string; mode: Exclude<VisualMode, 'hidden'>; tuning: StartupTuning; reducedMotion: boolean; preview: boolean }
  let mode = $state<VisualMode>('hidden'), tuning = $state<Readonly<StartupTuning>>(DEFAULT_TUNING);
  let elapsed = $state(0), width = $state(300), height = $state(300), reducedMotion = $state(false);
  let prepared = $state(false), phaseLabel = $state('Waiting for update…'), error = $state('');
  let root: HTMLDivElement;

  onMount(() => {
    let disposed = false, raf = 0, origin = 0, offset = 0, paused = false, lastSample = 0;
    let config: Config | null = null, ready = false;
    type Control = { action: string; elapsedMs?: number; tuning?: StartupTuning; label?: string; error?: string };
    const pendingControls: Control[] = [];
    function applyControl(control: Control) {
      if (control.action === 'pause') { paused = true; cancelAnimationFrame(raf); }
      if (control.action === 'resume') { origin = performance.now(); offset = elapsed; paused = false; cancelAnimationFrame(raf); raf = requestAnimationFrame(frame); }
      if (control.action === 'seek') { elapsed = Math.max(0, control.elapsedMs ?? 0); offset = elapsed; origin = performance.now(); paused = true; cancelAnimationFrame(raf); }
      if (control.action === 'tune' && control.tuning) tuning = validateTuning(control.tuning);
      if (control.action === 'status') { phaseLabel = control.label ?? ''; error = control.error ?? ''; }
    }
    const unlisten: Array<() => void> = [];
    const runId = new URL(location.href).searchParams.get('runId');
    const resize = new ResizeObserver(([entry]) => { width = entry.contentRect.width; height = entry.contentRect.height; }); resize.observe(root);
    async function finish() {
      if (!config || disposed) return;
      prepared = false;
      try { await invoke(config.preview ? 'preview_finish' : 'finish_intro', { runId: config.runId, revision: null }); }
      catch (error) { console.error('Finish startup surface:', error); }
    }
    function frame(now: number) {
      raf = 0; if (disposed || paused || !config) return;
      elapsed = Math.max(0, now - origin + offset);
      const duration = mode === 'compact-intro' ? REDUCED_INTRO_MS : introDuration(tuning);
      if ((mode === 'cinematic' || mode === 'compact-intro') && elapsed >= duration) { void finish(); return; }
      if (config.preview && now - lastSample >= 100) {
        lastSample = now;
        void invoke('preview_sample', { runId: config.runId, elapsedMs: elapsed, paused }).catch(() => {});
      }
      if (mode !== 'compact-error' && !(reducedMotion && mode === 'compact-wait')) raf = requestAnimationFrame(frame);
    }
    async function initialize() {
      if (!runId) return;
      config = await invoke<Config>('intro_config', { runId });
      if (disposed) return;
      tuning = validateTuning(config.tuning); mode = config.mode; reducedMotion = config.reducedMotion;
      if (config.preview) {
        const stop = await listen<{ runId: string; control: Control }>('startup-preview-control', event => {
          if (disposed || event.payload.runId !== config?.runId) return;
          // Active can reach the owner before this readiness reply reaches us.
          // Apply early pause/seek only after initializing the native time origin.
          if (!ready) pendingControls.push(event.payload.control);
          else applyControl(event.payload.control);
        });
        if (disposed) { stop(); return; } unlisten.push(stop);
      }
      // DOM flush works while hidden; waiting for a hidden RAF can deadlock startup.
      await tick();
      // Hidden webviews may defer ResizeObserver delivery. Force layout so the
      // very first visible pose already uses the full monitor-sized viewport.
      const bounds = root.getBoundingClientRect();
      width = Math.max(1, bounds.width); height = Math.max(1, bounds.height);
      await tick();
      const result = await invoke<{ shown: boolean; elapsedMs: number }>(config.preview ? 'preview_ready' : 'intro_ready', { runId });
      if (disposed || !result.shown) return;
      elapsed = result.elapsedMs; origin = performance.now(); offset = elapsed; prepared = true; ready = true;
      pendingControls.splice(0).forEach(applyControl);
      cancelAnimationFrame(raf);
      if (!paused) raf = requestAnimationFrame(frame);
    }
    void initialize().catch(error => { console.error('Prepare startup surface:', error); void finish(); });
    return () => { disposed = true; cancelAnimationFrame(raf); resize.disconnect(); unlisten.forEach(stop => stop()); };
  });
</script>

<div class:compact={mode.startsWith('compact')} class="surface" bind:this={root} style:opacity={prepared ? 1 : 0}>
  <StartupVisual {mode} {elapsed} {tuning} {width} {height} {reducedMotion} />
  {#if mode === 'compact-wait' || mode === 'compact-error'}<p class="status">{mode === 'compact-error' ? error : phaseLabel}</p>{/if}
</div>

<style>
  :global(html), :global(body) { margin: 0; padding: 0; background: transparent !important; overflow: hidden; }
  .surface { position: fixed; inset: 0; pointer-events: none; }
  .compact { background: #000; color: #ddd; }
  .status { position: absolute; bottom: 18px; left: 15px; right: 15px; margin: 0; text-align: center; font: 11px monospace; }
</style>
