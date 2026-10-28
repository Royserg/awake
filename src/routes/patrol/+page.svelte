<script lang="ts">
  import { onMount } from 'svelte';
  import { createPatrolClient } from '../../lib/patrol';
  import { reviewConfirmed } from '../../lib/patrol-state';
  import type { PatrolClient, Snapshot, Point, XY } from '../../lib/patrol-state';
  let client: PatrolClient | null = null;
  let snapshot = $state<Snapshot | null>(null);
  let error = $state<string | null>(null);
  let frozen = $state(false);
  let pauseSeconds = $state(10);
  let drawing: HTMLDivElement;
  let drag: { pointer: number; rect: DOMRect } | null = null;
  let localError = $state<string | null>(null);
  const setup = $derived(snapshot?.setup);
  const caller = $derived(snapshot?.caller);
  const mapping = $derived(caller?.mapping);
  function update() { snapshot = client?.snapshot ?? null; error = client?.error ?? null; frozen = client?.frozen ?? true; if (snapshot?.setup) pauseSeconds = snapshot.setup.pauseMs / 1000; }
  onMount(() => { client = createPatrolClient(update); void client.connect().catch(e => { error = String(e); }); return () => { client?.dispose(); }; });
  function act(promise: Promise<void> | undefined) { localError = null; void promise?.catch(e => { localError = typeof e?.message === 'string' ? e.message : 'The edit could not be completed'; }); }
  function sample(event: PointerEvent, rect: DOMRect) { return { u: (event.clientX - rect.left) / rect.width, v: (event.clientY - rect.top) / rect.height }; }
  function down(event: PointerEvent) {
    if (event.button !== 0 || frozen || !client || !drawing || (event.target as Element).closest('.panel')) return;
    const marker = (event.target as Element).closest('[data-point]');
    const pointId = marker?.getAttribute('data-point');
    const rect = drawing.getBoundingClientRect();
    if (pointId) { if (!client.begin(pointId)) return; drawing.setPointerCapture(event.pointerId); drag = { pointer: event.pointerId, rect }; }
    else { const p = sample(event, rect); act(client.add(p.u, p.v)); }
    event.preventDefault();
  }
  function move(event: PointerEvent) { if (!drag || drag.pointer !== event.pointerId) return; const p = sample(event, drag.rect); client?.move(p.u, p.v); }
  function up(event: PointerEvent) { if (!drag || drag.pointer !== event.pointerId) return; const p = sample(event, drag.rect); drag = null; client?.release(p.u, p.v); if (drawing.hasPointerCapture(event.pointerId)) drawing.releasePointerCapture(event.pointerId); }
  function lost() { if (drag) { drag = null; client?.captureLost(); } }
  function position(point: Point): XY | null { const display = setup?.displays.find(d => d.displayId === point.displayId && d.mirrorOf === null); return display ? { x: display.nativeBounds.x + point.x, y: display.nativeBounds.y + point.y } : null; }
  const positions = $derived(setup?.points.map(position) ?? []);
  function key(event: KeyboardEvent) { if (event.key === 'Escape') { event.preventDefault(); act(client?.cancel()); } }
</script>

<svelte:window onkeydown={key} />
<div class="surface" bind:this={drawing} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={lost} onlostpointercapture={lost} role="application" aria-label="Cursor patrol route editor">
  {#if snapshot && setup && caller && mapping}
    <svg class="route" viewBox={`${mapping.origin.x} ${mapping.origin.y} ${mapping.width} ${mapping.height}`} preserveAspectRatio="none" aria-label="Ordered cursor route">
      {#each positions as p, i}
        {@const next = positions[(i + 1) % positions.length]}
        {#if p && next && positions.length > 1}<line x1={p.x} y1={p.y} x2={next.x} y2={next.y} vector-effect="non-scaling-stroke" />{/if}
      {/each}
    </svg>
    {#each setup.points as point, i (point.pointId)}
      {@const p = positions[i]}
      {#if p && p.x >= mapping.origin.x && p.y >= mapping.origin.y && p.x < mapping.origin.x + mapping.width && p.y < mapping.origin.y + mapping.height}
        <button class="marker" class:invalid={setup.invalidPoints.some(v => v.pointId === point.pointId)} data-point={point.pointId} style:left={`${100 * (p.x - mapping.origin.x) / mapping.width}%`} style:top={`${100 * (p.y - mapping.origin.y) / mapping.height}%`} aria-label={`Drag point ${i + 1}`} disabled={frozen}>{i + 1}</button>
      {/if}
    {/each}
    {#if caller.isControlPanel}
      <section class="panel" aria-label="Cursor Patrolling settings">
        <div class="heading"><h1>Cursor Patrolling</h1><button class="close" onclick={() => act(client?.cancel())} aria-label="Cancel setup">×</button></div>
        <p>Click any screen to add points. Drag a point to adjust it. The cursor follows the numbered loop.</p>
        <label class="pause">Pause at each point <span><input type="number" min="1" max="3600" step="1" bind:value={pauseSeconds} disabled={frozen} onchange={() => { if (Number.isInteger(pauseSeconds) && pauseSeconds >= 1 && pauseSeconds <= 3600) act(client?.pause(pauseSeconds * 1000)); else localError = 'Enter a whole number of seconds from 1 to 3600'; }} /> seconds</span></label>
        <ol>{#each setup.points as point, i (point.pointId)}<li class:invalid={setup.invalidPoints.some(v => v.pointId === point.pointId)}><span><strong>{i + 1}</strong> {setup.displays.find(d => d.displayId === point.displayId)?.name ?? 'Disconnected display'}{setup.invalidPoints.some(v => v.pointId === point.pointId) ? ' — place again or remove' : ''}</span><button disabled={frozen} onclick={() => act(client?.remove(point.pointId))} aria-label={`Remove point ${i + 1}`}>Remove</button></li>{/each}</ol>
        {#if setup.points.length < 2}<p class="hint">Add at least two distinct points to begin.</p>{/if}
        {#if setup.review.required}<div class="review"><p>The display layout changed. Check all points before restarting.</p><button disabled={frozen || !setup.routeValidation.valid || reviewConfirmed(setup)} onclick={() => act(client?.confirm())}>{reviewConfirmed(setup) ? 'Points reviewed' : 'I have reviewed these points'}</button></div>{/if}
        {#if snapshot.availability.state !== 'ready'}<p class="hint">{snapshot.availability.message ?? 'Preparing input monitoring…'}</p>{#if snapshot.availability.state !== 'checking'}<button disabled={frozen} onclick={() => act(client?.permissions())}>Enable input access / Retry</button>{/if}{/if}
        {#if setup.overlays.some(o => o.state === 'incomplete' || o.state === 'unresponsive')}<p class="warning">An edit needs attention. Drag the affected point again, remove it, or reopen setup.</p>{/if}
        {#if error || localError || snapshot.status.lastFailure}<p class="warning" role="alert">{localError ?? error ?? snapshot.status.lastFailure?.message}</p>{/if}
        <footer><button onclick={() => act(client?.cancel())}>Cancel</button><button class="start" disabled={frozen || !setup.routeValidation.valid || !reviewConfirmed(setup) || snapshot.availability.state !== 'ready' || setup.overlays.some(o => ['initializing','incomplete','unresponsive'].includes(o.state))} onclick={() => act(client?.start())}>{snapshot.status.startStage === 'preparingEdits' ? 'Finishing edits…' : snapshot.status.startStage === 'arming' ? 'Starting…' : 'Start'}</button></footer>
        <small>Esc or moving the mouse stops patrol. Awake stays on.</small>
      </section>
    {:else}<div class="screen-note">Click to add points · Drag to adjust · Esc cancels setup</div>{/if}
  {:else}<section class="panel"><h1>Cursor Patrolling</h1><p>{error ?? 'Preparing your screens…'}</p></section>{/if}
</div>

<style>
  :global(html), :global(body) { margin: 0; width: 100%; height: 100%; overflow: hidden; background: transparent; }
  :global(*) { box-sizing: border-box; }
  .surface { position: fixed; inset: 0; background: rgba(12,20,34,.30); cursor: crosshair; color: #ebf3ff; font: 14px system-ui, sans-serif; touch-action: none; user-select: none; }
  .route { position: absolute; width: 100%; height: 100%; pointer-events: none; overflow: hidden; }
  line { stroke: #84c6ff; stroke-width: 2; stroke-dasharray: 7 5; opacity: .85; }
  button, input { font: inherit; }
  button { cursor: pointer; border: 1px solid #526780; background: #26394e; color: #f5f8ff; border-radius: 7px; padding: 7px 12px; }
  button:hover { background: #354f6c; } button:disabled { opacity: .5; cursor: default; }
  .marker { position: absolute; transform: translate(-50%,-50%); width: 34px; height: 34px; padding: 0; border: 2px solid white; border-radius: 50%; background: #1778bd; box-shadow: 0 2px 12px #00182999; cursor: grab; font-weight: 700; z-index: 1; }
  .marker:active { cursor: grabbing; }
  .panel { position: absolute; right: 24px; top: 24px; width: min(390px,calc(100% - 48px)); max-height: calc(100% - 48px); overflow: auto; padding: 22px; border: 1px solid #66809b; border-radius: 16px; background: #132333f5; box-shadow: 0 12px 50px #0005; cursor: default; z-index: 2; }
  .heading { display: flex; justify-content: space-between; align-items: center; } h1 { font-size: 22px; margin: 0; } p { line-height: 1.5; color: #c7d6e7; } .close { border: 0; padding: 0 6px; font-size: 26px; background: none; }
  .pause { display: flex; flex-direction: column; gap: 8px; } input { width: 90px; border: 1px solid #66809b; border-radius: 6px; padding: 7px; background: #0c1825; color: white; }
  ol { list-style: none; padding: 0; max-height: 190px; overflow: auto; } li { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin: 8px 0; } li strong { display: inline-block; width: 23px; color: #9dd6ff; } li button { padding: 4px 7px; font-size: 12px; } .invalid, .warning { color: #ffb8a0; } .hint { font-size: 13px; } .review { border-top: 1px solid #44566a; padding-bottom: 12px; } footer { display: flex; justify-content: flex-end; gap: 10px; margin-top: 20px; } .start { background: #167bbb; border-color: #8ccfff; } small { display: block; margin-top: 15px; line-height: 1.5; color: #a9bfd5; } .screen-note { position: absolute; bottom: 24px; left: 24px; padding: 12px 18px; border-radius: 10px; background: #132333ee; pointer-events: none; }
</style>
