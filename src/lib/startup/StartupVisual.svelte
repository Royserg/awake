<script lang="ts">
  import { onMount } from 'svelte';
  import { PixelAnimation } from './pixel-eye.ts';
  import { sampleStartupFrame } from './frame.ts';
  import { DEFAULT_TUNING, MAX_WAVE_CELLS } from './tuning.ts';
  import type { StartupTuning, VisualMode } from './types.ts';

  let { mode, elapsed = 0, tuning = DEFAULT_TUNING, width = 1280, height = 800, reducedMotion = false }: {
    mode: VisualMode; elapsed?: number; tuning?: Readonly<StartupTuning>;
    width?: number; height?: number; reducedMotion?: boolean;
  } = $props();
  let container: HTMLDivElement;
  let animation = $state<PixelAnimation | null>(null);
  const slots = Array.from({ length: MAX_WAVE_CELLS }, (_, index) => index);
  let frame = $derived(sampleStartupFrame(elapsed, mode, tuning, width, height, reducedMotion));
  let cell = $derived(mode === 'cinematic' ? tuning.introCellPx : 15);

  onMount(() => {
    const renderer = new PixelAnimation({ container, background: 'transparent', motion: 'startup', autoPlay: false });
    animation = renderer;
    return () => { renderer.destroy(); animation = null; };
  });
  $effect(() => { animation?.seek(frame.eyeProgress); });
</script>

<div class="visual" data-mode={mode} data-phase={frame.phase} style:opacity={frame.opacity} aria-hidden="true">
  <div class="backdrop" style:background={`radial-gradient(ellipse at center, rgba(0,0,0,${frame.backdropCenter}) 0%, rgba(0,0,0,${frame.backdropEdge}) 100%)`}></div>
  <svg class="wave" viewBox={`0 0 ${width} ${height}`} shape-rendering="crispEdges" style:opacity={frame.waveOpacity}>
    {#each slots as index (index)}
      <rect x={frame.waveCells[index]?.x ?? 0} y={frame.waveCells[index]?.y ?? 0}
        width={cell} height={cell} fill="#f2f2ee" display={index < frame.waveCells.length ? undefined : 'none'} />
    {/each}
  </svg>
  <div class="eye" style:width={`${16 * cell}px`} style:height={`${16 * cell}px`}
    style:transform={`translate(-50%, calc(-50% + ${frame.eyeOffsetPx}px))`} bind:this={container}></div>
</div>

<style>
  .visual, .backdrop, .wave { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  .visual { overflow: hidden; }
  .eye { position: absolute; left: 50%; top: 50%; image-rendering: pixelated; }
</style>
