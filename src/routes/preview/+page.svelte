<script lang="ts">
  import { onMount } from 'svelte';
  import StartupVisual from '../../lib/startup/StartupVisual.svelte';
  import { StartupPreview, PREVIEW_SCENARIOS, type PreviewScenario } from '../../lib/startup/preview-driver.ts';
  import { DEFAULT_TUNING, introDuration, validateTuning } from '../../lib/startup/tuning.ts';
  import type { StartupSnapshot, StartupTuning, VisualMode } from '../../lib/startup/types.ts';
  import { sampleStartupFrame } from '../../lib/startup/frame.ts';
  import { NativePresentation } from '../../lib/startup/native-presentation.ts';

  let scenario = $state<PreviewScenario>('ordinary');
  let tuning = $state<Readonly<StartupTuning>>(DEFAULT_TUNING);
  let draft = $state<StartupTuning>({ ...DEFAULT_TUNING });
  let validationError = $state('');
  let reducedMotion = $state(false);
  let paused = $state(false);
  let mode = $state<VisualMode>('hidden');
  let elapsed = $state(0);
  let snapshot = $state<Readonly<StartupSnapshot> | null>(null);
  let trace = $state<string[]>([]);
  let notice = $state('');
  let nativeAvailable = $state(false), nativeEnabled = $state(false), nativeStatus = $state('');
  let native: NativePresentation | undefined;
  let lastNativeStatus = '';
  let viewport = $state('stage');
  let stageWidth = $state(900), stageHeight = $state(560);
  let stage: HTMLDivElement;
  let session = $state.raw<StartupPreview | null>(null);
  let unsubscribe = () => {};
  let raf = 0, lastTime = 0;

  let width = $derived(viewport === 'stage' ? stageWidth : Number(viewport.split('x')[0]));
  let height = $derived(viewport === 'stage' ? stageHeight : Number(viewport.split('x')[1]));
  let scale = $derived(Math.min(stageWidth / width, stageHeight / height));
  let compact = $derived(mode.startsWith('compact'));
  let frame = $derived(sampleStartupFrame(elapsed, mode, tuning, width, height, reducedMotion));
  let timelineMax = $derived(mode === 'compact-wait' || mode === 'compact-error' ? tuning.idleCycleMs : introDuration(tuning));
  let timelinePosition = $derived(mode === 'compact-wait' ? elapsed % timelineMax : Math.min(elapsed, timelineMax));
  let progressPercent = $derived(snapshot?.progress?.total ? Math.round(snapshot.progress.downloaded / snapshot.progress.total * 100) : null);
  let statusText = $derived(snapshot?.updatePhase === 'installing' ? 'Installing update…'
    : snapshot?.updatePhase === 'restarting' ? 'Ready to restart'
    : snapshot?.stalled ? 'Still waiting for the download…'
    : snapshot?.updatePhase === 'downloading' ? `Downloading update${progressPercent === null ? '…' : ` · ${progressPercent}%`}`
    : snapshot?.error ?? '');

  type NumericKey = Exclude<keyof StartupTuning, 'waveEnabled'>;
  const groups: Array<{ title: string; fields: Array<{ key: NumericKey; label: string; min: number; max: number; step: number; unit: string }> }> = [
    { title: 'Timing', fields: [
      { key: 'openingMs', label: 'Opening', min: 500, max: 1200, step: 10, unit: 'ms' },
      { key: 'holdMs', label: 'Hold', min: 150, max: 900, step: 10, unit: 'ms' },
      { key: 'exitMs', label: 'Disappear', min: 150, max: 500, step: 10, unit: 'ms' },
    ] },
    { title: 'Eye & wave', fields: [
      { key: 'introCellPx', label: 'Pixel size', min: 8, max: 24, step: 1, unit: 'px' },
      { key: 'waveStartMs', label: 'Wave delay', min: 0, max: 700, step: 10, unit: 'ms' },
      { key: 'waveOpacity', label: 'Wave strength', min: 0, max: 0.4, step: 0.01, unit: '%' },
    ] },
    { title: 'Backdrop', fields: [
      { key: 'backdropCenterAlpha', label: 'Center dimming', min: 0, max: 0.7, step: 0.01, unit: '%' },
      { key: 'backdropEdgeAlpha', label: 'Edge dimming', min: 0, max: 0.7, step: 0.01, unit: '%' },
      { key: 'backdropEnterMs', label: 'Fade in', min: 0, max: 400, step: 10, unit: 'ms' },
    ] },
    { title: 'Closed-eye idle', fields: [
      { key: 'idleCycleMs', label: 'Cycle', min: 1800, max: 4000, step: 100, unit: 'ms' },
      { key: 'idleAmplitudeRows', label: 'Lift', min: 0, max: 1, step: 0.05, unit: 'rows' },
    ] },
  ];

  function refresh() {
    if (!session) return;
    mode = session.presentation.mode; elapsed = session.presentation.elapsed;
    snapshot = session.controller.snapshot; trace = [...session.updates.trace]; paused = session.paused;
    if (nativeEnabled && native) {
      const control = { action: 'status', label: statusText, error: snapshot.error ?? '' };
      const key = JSON.stringify(control);
      if (key !== lastNativeStatus) { lastNativeStatus = key; void native.control(control).catch(() => {}); }
    }
  }
  function tick(time: number) {
    raf = 0;
    if (!session || session.paused) return;
    session.advance(Math.max(0, time - lastTime)); lastTime = time;
    refresh();
    if (session.clock.pendingCount || mode !== 'hidden') raf = requestAnimationFrame(tick);
  }
  function startFrames() {
    if (raf) cancelAnimationFrame(raf);
    lastTime = performance.now(); raf = requestAnimationFrame(tick);
  }
  function stop() {
    cancelAnimationFrame(raf); raf = 0; unsubscribe();
    session?.dispose(); session = null; mode = 'hidden'; elapsed = 0; paused = false; snapshot = null;
  }
  function replay(nextScenario = scenario) {
    if (validationError) return;
    stop(); scenario = nextScenario; notice = '';
    session = new StartupPreview(scenario, tuning, reducedMotion, nativeEnabled ? native : undefined);
    unsubscribe = session.controller.subscribe(refresh);
    refresh(); startFrames();
  }
  function togglePause() {
    if (!session) return;
    if (session.paused) { session.resume(); startFrames(); }
    else { session.pause(); cancelAnimationFrame(raf); raf = 0; }
    refresh();
  }
  function seek(value: number) {
    if (!session) replay();
    session?.seek(value); cancelAnimationFrame(raf); raf = 0; refresh();
  }
  function applyTuning() {
    try { tuning = validateTuning({ ...draft }); validationError = ''; session?.tune(tuning); refresh(); }
    catch (error) { validationError = error instanceof Error ? error.message : String(error); }
  }
  function change(key: NumericKey, value: string) { draft[key] = Number(value); applyTuning(); }
  function reset() { draft = { ...DEFAULT_TUNING }; applyTuning(); replay(); }
  async function act(action: () => void) {
    action();
    // Also observe settled promise phases when the virtual clock is paused.
    for (let i = 0; i < 20; i++) await Promise.resolve();
    refresh();
    if (session && !session.paused && !raf) startFrames();
  }
  function simulateRestart() { replay('ordinary'); notice = 'New simulated launch · no update'; }
  function handoff() { stop(); notice = 'Simulated Windows installer handoff. Choose Simulate restart for the new launch.'; }
  async function expand() {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await stage.requestFullscreen();
  }
  async function copyTuning() {
    try { await navigator.clipboard.writeText(JSON.stringify(tuning, null, 2)); notice = 'Tuning copied. Values are still preview-only.'; }
    catch { notice = 'Clipboard unavailable. Select the tuning values below to copy.'; }
  }

  onMount(() => {
    reducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
    const resize = new ResizeObserver(([entry]) => { stageWidth = Math.round(entry.contentRect.width); stageHeight = Math.round(entry.contentRect.height); });
    resize.observe(stage);
    const keyboard = (event: KeyboardEvent) => {
      if ((event.target as HTMLElement)?.closest('input, select, textarea')) return;
      if (event.key.toLowerCase() === 'r') replay();
      if (event.code === 'Space' && !(event.target as HTMLElement)?.closest('button')) { event.preventDefault(); togglePause(); }
    };
    let disposed = false;
    void import('../../lib/startup/tauri-port.ts').then(async ({ isTauri, tauriPort }) => {
      if (!isTauri()) return;
      await tauriPort.invoke('preview_context');
      if (disposed) return;
      native = new NativePresentation(tauriPort, {
        scope: 'preview',
        clock: { now: () => performance.now(), schedule: (ms, callback) => { const timer = setTimeout(callback, ms); return () => clearTimeout(timer); } },
        prepare: async () => { lastNativeStatus = ''; },
        onStatus: (status, error) => { nativeStatus = error ? `${status}: ${error}` : status; },
        onSample: sample => { if (nativeEnabled && session) { session.presentation.syncElapsed(sample.elapsedMs); refresh(); } },
      });
      nativeAvailable = true;
    }).catch(() => { nativeStatus = 'Native replay is available from the debug Intro Preview menu.'; });
    window.addEventListener('keydown', keyboard); replay();
    return () => { disposed = true; stop(); resize.disconnect(); window.removeEventListener('keydown', keyboard); void native?.dispose().catch(() => {}); };
  });
</script>

<svelte:head><title>Awake · Startup preview</title><meta name="description" content="Tune Awake's pixel-eye opening and simulated update transitions." /></svelte:head>

<main>
  <header>
    <a class="brand" href="/preview" aria-label="Awake preview"><span class="brand-pixel"></span> AWAKE <span class="separator">/</span> <span class="muted">MOTION LAB</span></a>
    <span class="badge">PREVIEW ONLY</span>
  </header>
  <section class="heading">
    <div><p class="eyebrow">THE FIRST MOMENT</p><h1>A natural awakening.</h1><p class="subtitle">The same pixel eye. A little more life between frames.</p></div>
    <div class="duration"><strong>{(introDuration(tuning) / 1000).toFixed(2)}<span>s</span></strong><span>opening → hold → disappear</span></div>
  </section>
  <div class="workspace">
    <section class="preview-column" aria-label="Animation replay">
      <div class="stage-toolbar"><span class="eyebrow">{nativeEnabled ? 'NATIVE REPLAY + BROWSER STAGE' : 'BROWSER STAGE'}</span><div><select aria-label="Preview viewport" bind:value={viewport}>
        <option value="stage">Current stage · 1:1</option><option value="1280x800">1280 × 800</option><option value="1920x1080">1920 × 1080</option><option value="3840x2160">3840 × 2160</option><option value="3440x1440">3440 × 1440</option>
      </select><button class="small" onclick={expand}>Expand stage ↗</button></div></div>
      <div class="stage" bind:this={stage}>
        <button class="exit-stage" onclick={expand}>Return to controls ↙</button>
        <div class="scene" style:width={`${width}px`} style:height={`${height}px`} style:transform={`translate(-50%, -50%) scale(${scale})`}>
          <div class="desktop"><div class="desktop-bar"><span>AWAKE</span><span>Simulated desktop</span><span>◦ ◦ ◦</span></div><div class="desktop-window"><div class="window-bar"><i></i><i></i><i></i></div><div class="desktop-lines"><b></b><b></b><b></b><b></b></div></div><span class="desktop-word">Make room<br />for a moment.</span><div class="desktop-dock"><i></i><i></i><i></i><i></i><i></i></div></div>
          {#if compact}
            <div class="compact-window">
              <StartupVisual {mode} {elapsed} {tuning} width={300} height={300} {reducedMotion} />
              <div class="compact-status"><span class="compact-label">{snapshot?.version ?? 'AWAKE'}</span><p>{mode === 'compact-intro' ? '' : mode === 'compact-error' ? snapshot?.error : statusText}</p></div>
            </div>
          {:else}
            <StartupVisual {mode} {elapsed} {tuning} {width} {height} {reducedMotion} />
          {/if}
        </div>
        <div class="stage-caption"><span>{width} × {height} CSS px · {Math.round(scale * 100)}%</span><span>{mode === 'hidden' ? 'DESKTOP CLEAR' : frame.phase.toUpperCase()}</span></div>
      </div>
      <div class="transport">
        <div class="buttons"><button class="primary" onclick={() => replay()} disabled={!!validationError}>↻ Replay <kbd>R</kbd></button><button onclick={togglePause} disabled={!session}>{paused ? '▶ Resume' : 'Ⅱ Pause'}</button><button onclick={stop} disabled={!session}>■ Stop</button></div>
        <span class="timestamp">{Math.round(timelinePosition)} <span>/ {Math.round(timelineMax)} ms</span></span>
      </div>
      <label class="scrubber"><span class="sr-only">Visual timeline</span><input aria-label="Visual timeline" type="range" min="0" max={timelineMax} step="1" value={timelinePosition} oninput={event => seek(Number(event.currentTarget.value))} /></label>
      <div class="timeline-labels">{#if compact}<span>CLOSED</span><span>IDLE LIFT</span><span>REST</span>{:else}<span>OPEN</span><span>HOLD</span><span>DISAPPEAR</span>{/if}</div>
      <p class="stage-note">The backdrop here is simulated. Native replay uses separate desktop windows with a 30-second safety limit.</p>
      {#if nativeAvailable}<label class="toggle native-toggle"><span>Replay in native desktop windows · {nativeStatus || 'ready'}</span><input aria-label="Native desktop replay" type="checkbox" bind:checked={nativeEnabled} onchange={() => replay()} /></label>{/if}

      <section class="flows" aria-label="Simulated update transitions">
        <div class="flow-heading"><div><p class="eyebrow">BEYOND THE OPENING</p><h2>Rehearse the handoff.</h2></div><span class="badge">SIMULATED</span></div>
        <label class="scenario"><span>Launch scenario</span><select aria-label="Launch scenario" bind:value={scenario} onchange={() => replay()}>{#each Object.entries(PREVIEW_SCENARIOS) as [value, label]}<option {value}>{label}</option>{/each}</select></label>
        <div class="flow-state"><span>{snapshot?.updatePhase ?? 'stopped'}</span><span>{mode}</span><span>{snapshot?.hiddenByUser ? 'wait hidden' : 'presentation visible when needed'}</span></div>
        <div class="flow-actions">
          <button disabled={snapshot?.updatePhase !== 'downloading' || !session} onclick={() => act(() => session?.updates.downloadComplete())}>Download complete</button>
          <button disabled={snapshot?.updatePhase !== 'installing' || !session} onclick={() => act(() => session?.updates.installComplete())}>Install complete</button>
          <button disabled={!session || !['downloading', 'installing', 'restarting'].includes(snapshot?.updatePhase ?? '')} onclick={() => act(() => session?.updates.fail(snapshot?.updatePhase === 'installing' ? 'install' : snapshot?.updatePhase === 'restarting' ? 'restart' : 'download'))}>Fail current step</button>
          <button disabled={!session || !compact} onclick={() => act(() => session?.controller.hideWait())}>Hide wait</button>
          {#if snapshot?.failurePhase === 'downloading'}<button disabled={snapshot.busy} onclick={() => act(() => session?.controller.retryDownload())}>Try again</button>{/if}
          {#if snapshot?.failurePhase === 'restarting'}<button disabled={snapshot.busy} onclick={() => act(() => { void session?.controller.retryRestart(); })}>Restart</button>{/if}
          <button disabled={!session || !['installing', 'restarting'].includes(snapshot?.updatePhase ?? '')} onclick={handoff}>Windows handoff</button>
          <button onclick={simulateRestart}>Simulate restart</button>
        </div>
        <p class="flow-note">{snapshot?.failurePhase === 'installing' ? 'Installation failure offers Hide only; a potentially partial install is not retried.' : 'Pause freezes simulated time. Scrubbing samples the eye without advancing the update. Restart creates a new simulated launch.'}</p>
        <details><summary>Simulated operation trace <span>{trace.length}</span></summary><code>{trace.length ? trace.join(' → ') : 'No operations'}</code></details>
      </section>
      {#if notice}<p class="notice" role="status">{notice}</p>{/if}
    </section>

    <aside aria-label="Motion tuning">
      <div class="tuning-heading"><h2>Fine tune</h2><button class="text-button" onclick={reset}>Reset</button></div>
      <p class="tuning-note">Changes stay in this preview. Replay to feel the whole sequence.</p>
      <label class="toggle"><span>Reduced motion</span><input type="checkbox" bind:checked={reducedMotion} onchange={() => replay()} /></label>
      {#each groups as group}
        <fieldset><legend>{group.title}</legend>
          {#each group.fields as field}
            <label class="parameter"><span>{field.label}<output>{field.unit === '%' ? `${Math.round(draft[field.key] * 100)}%` : `${draft[field.key]} ${field.unit}`}</output></span><input aria-label={field.label} type="range" min={field.min} max={field.max} step={field.step} value={draft[field.key]} oninput={event => change(field.key, event.currentTarget.value)} /></label>
          {/each}
          {#if group.title === 'Eye & wave'}<label class="toggle"><span>Pixel shock wave</span><input type="checkbox" bind:checked={draft.waveEnabled} onchange={applyTuning} /></label>{/if}
        </fieldset>
      {/each}
      {#if validationError}<p class="validation" role="alert">{validationError}. The last valid tuning remains visible.</p>{/if}
      <button class="copy" onclick={copyTuning} disabled={!!validationError}>Copy tuning ↗</button>
      <details class="values"><summary>View tuning values</summary><textarea readonly aria-label="Tuning values" value={JSON.stringify(tuning, null, 2)} rows="14"></textarea></details>
    </aside>
  </div>
  <footer><span>16 × 16 ARTWORK · MONOCHROME · EVERY LAUNCH</span><span>Browser simulation · native replay in the debug Intro Preview window</span></footer>
</main>

<style>
  :global(body) { margin: 0; background: #111111; color: #dedede; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }
  :global(*) { box-sizing: border-box; }
  main { max-width: 1540px; margin: auto; padding: 0 44px; }
  header { height: 78px; border-bottom: 1px solid #2b2b2b; display: flex; align-items: center; justify-content: space-between; }
  .brand { color: inherit; text-decoration: none; font: 600 12px monospace; letter-spacing: 2px; display: flex; align-items: center; gap: 12px; }
  .brand-pixel { width: 13px; height: 9px; background: #e4e4e4; box-shadow: 3px 0 #e4e4e4, -3px 0 #e4e4e4; position: relative; }
  .brand-pixel::after { content: ''; position: absolute; width: 5px; height: 5px; background: #111111; left: 4px; top: 2px; }
  .separator { margin: 0 10px; color: #535353; } .muted { color: #8d8d8d; }
  .badge { font: 9px monospace; letter-spacing: 1.4px; color: #9a9a9a; border: 1px solid #3b3b3b; padding: 7px 9px; border-radius: 3px; white-space: nowrap; }
  .heading { display: flex; align-items: end; justify-content: space-between; padding: 42px 0 32px; }
  .eyebrow { font: 10px monospace; letter-spacing: 2px; color: #909090; margin: 0 0 13px; }
  h1 { margin: 0; font-size: clamp(28px, 3vw, 43px); font-weight: 450; letter-spacing: -1.6px; }
  .subtitle { font-size: 13px; color: #949494; margin: 12px 0 0; } .duration { display: grid; gap: 8px; text-align: right; color: #7f7f7f; font: 10px monospace; }
  .duration strong { color: #e5e5e5; font: 35px monospace; letter-spacing: -2px; } .duration strong span { font-size: 18px; color: #8a8a8a; margin-left: 4px; }
  .workspace { display: grid; grid-template-columns: minmax(0, 1fr) 278px; gap: 32px; } .preview-column { min-width: 0; }
  .stage-toolbar { display: flex; align-items: center; justify-content: space-between; padding: 0 0 12px; gap: 10px; } .stage-toolbar .eyebrow { margin: 0; }
  .stage-toolbar > div { display: flex; gap: 8px; }
  button, select { font-family: inherit; font-size: 11px; color: #c4c4c4; background: #1d1d1d; border: 1px solid #3c3c3c; border-radius: 4px; padding: 9px 11px; cursor: pointer; }
  button { font-size: 11px; } select { font-size: 11px; max-width: 100%; } button:hover:not(:disabled) { border-color: #a1a1a1; color: white; } button:focus-visible, select:focus-visible, input:focus-visible { outline: 2px solid #e0e0e0; outline-offset: 4px; }
  button:disabled { opacity: .35; cursor: default; } button.primary { background: #e1e1e1; border-color: #e1e1e1; color: #1c1c1c; font-weight: 600; } button.primary:hover:not(:disabled) { color: #1c1c1c; background: white; }
  kbd { font: 9px monospace; border: 1px solid #878787; padding: 1px 4px; margin-left: 12px; border-radius: 2px; }
  .stage { height: clamp(360px, 45vw, 620px); background: #252525; overflow: hidden; position: relative; border: 1px solid #474747; border-radius: 7px; }
  .stage:fullscreen { height: 100vh; width: 100vw; border: 0; border-radius: 0; }
  .exit-stage { display: none; position: absolute; z-index: 2; top: 42px; right: 16px; background: #151515cc; }
  .stage:fullscreen .exit-stage { display: block; }
  .scene { position: absolute; left: 50%; top: 50%; transform-origin: center; overflow: hidden; }
  .desktop { position: absolute; inset: 0; background: radial-gradient(ellipse at 75% 30%, #a8a8a8, #797979 55%, #555555); color: #373737; }
  .desktop-bar { height: 31px; background: #bfbfbfaa; display: flex; align-items: center; justify-content: space-between; font: 10px monospace; padding: 0 18px; }
  .desktop-window { position: absolute; width: 31%; height: 34%; left: 9%; top: 23%; background: #bbbbbb; border: 1px solid #4f4f4f55; border-radius: 7px; box-shadow: 0 12px 40px #2d2d2d22; }
  .window-bar { border-bottom: 1px solid #74747455; height: 27px; display: flex; gap: 5px; align-items: center; padding: 0 10px; } .window-bar i { width: 5px; height: 5px; border: 1px solid #747474; border-radius: 50%; }
  .desktop-lines { padding: 20px; display: grid; gap: 11px; } .desktop-lines b { width: 80%; height: 3px; background: #8f8f8f; opacity: .7; } .desktop-lines b:first-child { width: 50%; height: 6px; margin-bottom: 10px; } .desktop-lines b:last-child { width: 60%; }
  .desktop-word { position: absolute; right: 9%; top: 30%; font-size: clamp(20px, 3vw, 42px); font-weight: 400; line-height: 1.2; letter-spacing: -1px; opacity: .5; }
  .desktop-dock { position: absolute; bottom: 22px; left: 50%; transform: translateX(-50%); border-radius: 9px; padding: 9px; display: flex; gap: 9px; background: #c9c9c955; border: 1px solid #c9c9c955; } .desktop-dock i { width: 26px; height: 26px; border-radius: 5px; background: #717171; border: 1px solid #58585855; }
  .compact-window { position: absolute; width: 300px; height: 300px; left: 50%; top: 50%; transform: translate(-50%, -50%); background: #0a0a0a; box-shadow: 0 24px 80px #0006; }
  .compact-status { position: absolute; bottom: 15px; left: 15px; right: 15px; text-align: center; color: #a7a7a7; font: 11px monospace; } .compact-status p { margin: 7px 0 0; } .compact-label { font-size: 9px; letter-spacing: 1px; color: #777777; }
  .stage-caption { position: absolute; bottom: 12px; left: 14px; right: 14px; display: flex; justify-content: space-between; color: #e7e7e7; font: 9px monospace; letter-spacing: .6px; pointer-events: none; text-shadow: 0 1px 4px #000a; }
  .transport { display: flex; align-items: center; justify-content: space-between; padding: 17px 0 12px; gap: 8px; } .buttons { display: flex; gap: 7px; } .timestamp { font: 11px monospace; color: #d4d4d4; } .timestamp span { color: #727272; }
  input[type='range'] { width: 100%; accent-color: #d4d4d4; height: 12px; cursor: pointer; margin: 5px 0 0; } .scrubber { display: block; } .timeline-labels { display: flex; justify-content: space-between; font: 8px monospace; color: #777777; letter-spacing: 1px; margin-top: 4px; }
  .stage-note { font-size: 10px; line-height: 1.5; color: #7a7a7a; margin: 20px 0 30px; }
  aside { border-left: 1px solid #2f2f2f; padding-left: 26px; } h2 { margin: 0; font-size: 16px; font-weight: 500; letter-spacing: -.3px; } .tuning-heading { display: flex; align-items: center; justify-content: space-between; }
  .text-button { background: none; border: none; padding: 0; color: #969696; text-decoration: underline; text-underline-offset: 4px; } .tuning-note { color: #878787; font-size: 11px; line-height: 1.6; margin: 12px 0 17px; }
  fieldset { border: 0; border-top: 1px solid #303030; padding: 15px 0 5px; margin: 22px 0 0; min-width: 0; } legend { font: 10px monospace; text-transform: uppercase; letter-spacing: 1px; color: #a4a4a4; padding-right: 10px; }
  .parameter { display: block; margin-bottom: 13px; } .parameter > span { display: flex; justify-content: space-between; color: #b3b3b3; font-size: 11px; } output { font: 10px monospace; color: #858585; }
  .toggle { display: flex; align-items: center; justify-content: space-between; color: #adadad; font-size: 11px; gap: 8px; } input[type='checkbox'] { accent-color: #d8d8d8; }
  .copy { width: 100%; margin-top: 15px; } .values { margin-top: 12px; } textarea { width: 100%; margin-top: 12px; background: #191919; color: #afafaf; border: 1px solid #434343; font: 10px monospace; padding: 9px; resize: vertical; }
  .flows { border-top: 1px solid #343434; padding-top: 25px; } .flow-heading { display: flex; justify-content: space-between; align-items: center; margin-bottom: 22px; } .flow-heading .eyebrow { margin-bottom: 8px; }
  .scenario { display: flex; align-items: center; gap: 18px; font-size: 11px; color: #9a9a9a; } .scenario select { flex: 1; } .flow-state { display: flex; gap: 14px; flex-wrap: wrap; font: 9px monospace; margin: 15px 0; color: #a3a3a3; } .flow-state span::before { content: '· '; color: #cacaca; }
  .flow-actions { display: flex; flex-wrap: wrap; gap: 7px; } .flow-actions button { font-size: 10px; } .flow-note { font-size: 10px; line-height: 1.6; color: #878787; } details { font: 10px monospace; color: #8d8d8d; } summary { cursor: pointer; } summary span { margin-left: 9px; } details code { display: block; margin-top: 13px; line-height: 1.8; overflow-wrap: anywhere; }
  .validation, .notice { font-size: 11px; line-height: 1.6; color: #cfcfcf; padding: 12px; border: 1px solid #525252; background: #242424; border-radius: 3px; }
  footer { border-top: 1px solid #313131; display: flex; justify-content: space-between; gap: 15px; margin-top: 40px; padding: 22px 0 28px; color: #717171; font: 9px monospace; letter-spacing: 1px; }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); border: 0; }
  @media (max-width: 920px) { main { padding: 0 20px; } .workspace { grid-template-columns: minmax(0, 1fr); } aside { border-left: 0; border-top: 1px solid #2f2f2f; padding: 25px 0 0; } .heading { padding-top: 30px; } .stage { height: 480px; } footer { flex-direction: column; } }
  @media (max-width: 560px) { .brand { font-size: 10px; gap: 7px; letter-spacing: 1px; } .separator { margin: 0 3px; } .duration { display: none; } .stage-toolbar { align-items: start; } .stage-toolbar > div { flex-direction: column; } .stage { height: 370px; } .timestamp { font-size: 9px; } kbd { display: none; } .transport button { padding: 9px; } }
</style>
