<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { isTauri, tauriPort } from '../lib/startup/tauri-port.ts';
  import { desktopClock, desktopUpdates } from '../lib/startup/tauri-driver.ts';
  import { StartupController } from '../lib/startup/controller.ts';
  import { NativePresentation } from '../lib/startup/native-presentation.ts';
  import StartupVisual from '../lib/startup/StartupVisual.svelte';
  import type { StartupSnapshot, VisualMode } from '../lib/startup/types.ts';

  let snapshot = $state<Readonly<StartupSnapshot> | null>(null);
  let mode = $state<VisualMode>('hidden'), elapsed = $state(0), reducedMotion = $state(false), browserOnly = $state(false);
  let controller = $state.raw<StartupController | null>(null);
  let percent = $derived(snapshot?.progress?.total ? Math.min(100, Math.round(snapshot.progress.downloaded / snapshot.progress.total * 100)) : null);
  let label = $derived(snapshot?.updatePhase === 'installing' ? 'Installing update…'
    : snapshot?.updatePhase === 'restarting' ? 'Restarting Awake…'
    : snapshot?.stalled ? 'Still waiting for the download…'
    : percent === null ? 'Downloading update…' : `Downloading update · ${percent}%`);

  onMount(() => {
    if (!isTauri()) { browserOnly = true; return; }
    reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
    let raf = 0, origin = 0;
    function stopFrames() { cancelAnimationFrame(raf); raf = 0; }
    function frame(now: number) { elapsed = now - origin; raf = requestAnimationFrame(frame); }
    const presentation = new NativePresentation(tauriPort, {
      scope: 'production', clock: desktopClock, reducedMotion,
      async prepare(next) { stopFrames(); mode = next === 'cinematic' ? 'hidden' : next; elapsed = 0; await tick(); },
      visible(next) {
        stopFrames(); mode = next === 'cinematic' ? 'hidden' : next; elapsed = 0; origin = performance.now();
        if (next === 'compact-wait' && !reducedMotion) raf = requestAnimationFrame(frame);
      },
      onUserHide: () => controller?.hideWait(),
      onCompatibilityShow: () => controller?.showStaticIntro(),
    });
    const owner = new StartupController({ clock: desktopClock, updates: desktopUpdates, presentation, reducedMotion, beforeInstall: tick, onVisualError: error => console.warn('Startup presentation:', error) });
    controller = owner;
    const unsubscribe = owner.subscribe(next => { snapshot = next; });
    owner.start();
    return () => {
      unsubscribe(); stopFrames(); owner.dispose(); controller = null;
      void presentation.dispose().catch(error => console.warn('Startup cleanup:', error));
    };
  });
</script>

<main>
  {#if browserOnly}
    <p class="browser">Awake runs in the desktop app.<br /><a href="/preview">Open the animation preview →</a></p>
  {:else}
    <StartupVisual {mode} {elapsed} {reducedMotion} width={300} height={300} />
    {#if mode === 'compact-wait' || mode === 'compact-error'}
      <section class="status" aria-live="polite">
        <span class="version">{snapshot?.version ? `Awake · v${snapshot.version}` : 'Awake'}</span>
        <p>{mode === 'compact-error' ? snapshot?.error : label}</p>
        <div class="actions">
          <button onclick={() => controller?.hideWait()}>Hide</button>
          {#if mode === 'compact-error' && snapshot?.failurePhase === 'downloading'}<button disabled={snapshot.busy} onclick={() => controller?.retryDownload()}>Try again</button>{/if}
          {#if mode === 'compact-error' && snapshot?.failurePhase === 'restarting'}<button disabled={snapshot.busy} onclick={() => controller?.retryRestart()}>Restart</button>{/if}
        </div>
      </section>
    {/if}
  {/if}
</main>

<style>
  :global(html), :global(body) { margin: 0; padding: 0; background: #000; overflow: hidden; }
  main { position: fixed; inset: 0; background: #000; color: #deded8; font: 11px monospace; }
  .status { position: absolute; left: 16px; right: 16px; bottom: 12px; text-align: center; }
  .version { font-size: 9px; letter-spacing: 1px; color: #888; }
  p { margin: 8px 0; line-height: 1.4; overflow-wrap: anywhere; max-height: 48px; overflow: auto; }
  .actions { display: flex; gap: 8px; justify-content: center; }
  button { border: 1px solid #555; background: #161616; color: #ddd; padding: 5px 11px; border-radius: 3px; font: 10px monospace; cursor: pointer; }
  button:disabled { opacity: .4; } button:focus-visible { outline: 2px solid #ddd; outline-offset: 2px; }
  .browser { padding: 30px; } a { color: #fff; display: inline-block; margin-top: 16px; }
</style>
