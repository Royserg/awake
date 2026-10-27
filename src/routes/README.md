# Frontend routes and splash animation

## Overview

This directory supplies the statically generated SvelteKit pages embedded by Tauri. The hidden root page owns updater work and compact states; `/intro` renders native-owned cinematic or preview surfaces. The timer page controls a backend timer.

## Interface

- [`+layout.ts`](./+layout.ts) exports `prerender = true` and `ssr = false` for the route tree.
- [`+page.svelte`](./+page.svelte) owns one `StartupController`, `NativePresentation`, and production updater adapter. On mount it starts checking independently of native visual preparation and renders compact wait/error/static states. It exports no props or named symbols.
- [`eye-splashscreen.ts`](./eye-splashscreen.ts) forwards the shared [pixel-eye renderer](../lib/startup/pixel-eye.ts) exports `PixelTransform`, `PixelLayerData`, `PixelObjectSnapshot`, `PixelAnimationOptions`, `PixelAnimationController`, and the implementing `PixelAnimation` class.
- Animation data contracts describe transforms, named layers with percentage keyframes, and pixel/rectangle/circle/path snapshots. Options accept a container, duration, speed, repeat mode, autoplay, and `onComplete`, `onFinished`, and `onFrame` callbacks.
- `PixelAnimationController` exposes readonly `svg`, `isPlaying`, `progress`, `duration`, and `speed`; `play`, `pause`, `stop`, `seek`, `setSpeed`, `setRepeatMode`, `setDirection`, `getLayer`, `getLayerNames`, and `destroy` control or inspect the SVG animation.
- `onFrame` receives progress on each playback tick. `onComplete` fires when playback wraps and on a completed `once` run. A completed `once` run also calls `onFinished` and dispatches `animation-finished` on `svg`, both with `{ progress, iteration }` detail.
- [Timer route contract](./timer/README.md): owns the `/timer` UI and the `get_timer_remaining_ms`, `set_timer_duration`, and `cancel_timer` IPC calls. Pages share the layout without passing props between each other.
- [Replay preview](./preview/README.md): `/preview` tunes shared startup visuals and exercises simulated update transitions using fake updates. Debug native replay mirrors visual commands into separate passive windows.
- `sampleEyeArtwork` exports a pure sample of the canonical layer/object artwork for renderer and regression tests. `PixelAnimationOptions` additionally accepts `background: 'black' | 'transparent'` and `motion: 'legacy' | 'startup'`; both default to legacy behavior.
- `EyeArtworkLayer` describes sampled layer `id`, `name`, `transform`, and `objectSnapshots`. Samplers do not create or modify SVG nodes.

## Invariants

- Tauri hosts a static frontend without server-side rendering. Browser-dependent animation setup runs in `onMount`.
- Startup uses the shared sampler: 850 ms opening, 600 ms hold, 300 ms exit. Reduced motion uses a 250 ms static compact intro. The legacy renderer defaults below remain available.
- `PixelAnimation` defaults to 1000 ms duration, speed 1, looping, normal direction, and zero progress. Its SVG uses a `0 0 256 256` viewBox and crisp edges.
- `play` is idempotent while playing; `stop` pauses and resets progress and iteration. `seek` clamps to 0–100 and `setSpeed` clamps to at least 0.1; constructor duration and speed are accepted without those clamps.
- Unknown layer names return `null`. `destroy` pauses playback, removes the SVG, and clears the layer map.
- Compact animation data expands into percentage-sorted layer keyframes. Pixel snapshots accept direct arrays or `pixels-rle-v1` payloads; decoding caches the expanded array in the snapshot. Objects introduced at later keyframes stay hidden before their first appearance, and `hold` easing preserves the preceding snapshots between keyframes.
- Startup sampling uses bounded cubic transform interpolation and fades newly born objects. Transparent mode removes only the named background layer. SVG object/pixel nodes are reused across seeks; transforms and geometry attributes are updated in place. Seeking still emits no playback completion callbacks.
- Root teardown removes observers, frame work, and presentation listeners. Pending updater resources close only after their operation settles. Rendered progress and recovery controls reflect controller state.
- [Passive surface](./intro/README.md) loads only validated native run configuration and acknowledges its prepared DOM before showing.

## Side Effects

- Animation construction creates SVG nodes and optionally appends them to the supplied DOM container. Playback changes the SVG and uses browser timing; pausing cancels a pending animation frame.
- The root checks, downloads, flushes the installing label, then installs. macOS/Linux request relaunch; Windows may exit into its installer before the install promise returns. Check failure leaves ordinary startup usable; download/restart failures offer scoped recovery.
- A confirmed update supersedes the overlay and prepares compact waiting before downloading. Hiding leaves the updater owner alive. Native cleanup and revision barriers reject stale completions.
- The root renders a compact black surface only when native presentation requests it; the intro route keeps its cinematic background transparent. The timer page applies its own global body styling and invokes backend timer operations.

## Verification

From the repository root, run `pnpm check`, `pnpm test:startup`, and `pnpm build`.
