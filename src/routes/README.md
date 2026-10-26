# Frontend routes and splash animation

## Overview

This directory supplies the statically generated SvelteKit pages embedded by Tauri. The root page shows the animated eye splash and checks for updates; the separate timer page controls a backend timer.

## Interface

- [`+layout.ts`](./+layout.ts) exports `prerender = true` and `ssr = false` for the route tree.
- [`+page.svelte`](./+page.svelte) is the splash page. On mount it creates and plays `PixelAnimation`, checks for updates, and invokes `hide_window` when the splash should close. It exports no props or named symbols.
- [`eye-splashscreen.ts`](./eye-splashscreen.ts) exports `PixelTransform`, `PixelLayerData`, `PixelObjectSnapshot`, `PixelAnimationOptions`, `PixelAnimationController`, and the implementing `PixelAnimation` class.
- Animation data contracts describe transforms, named layers with percentage keyframes, and pixel/rectangle/circle/path snapshots. Options accept a container, duration, speed, repeat mode, autoplay, and `onComplete`, `onFinished`, and `onFrame` callbacks.
- `PixelAnimationController` exposes readonly `svg`, `isPlaying`, `progress`, `duration`, and `speed`; `play`, `pause`, `stop`, `seek`, `setSpeed`, `setRepeatMode`, `setDirection`, `getLayer`, `getLayerNames`, and `destroy` control or inspect the SVG animation.
- `onFrame` receives progress on each playback tick. `onComplete` fires when playback wraps and on a completed `once` run. A completed `once` run also calls `onFinished` and dispatches `animation-finished` on `svg`, both with `{ progress, iteration }` detail.
- [Timer route contract](./timer/README.md): owns the `/timer` UI and the `get_timer_remaining_ms`, `set_timer_duration`, and `cancel_timer` IPC calls. Pages share the layout without passing props between each other.

## Invariants

- Tauri hosts a static frontend without server-side rendering. Browser-dependent animation setup runs in `onMount`.
- The splash requests a 1000 ms animation with repeat mode `once`, then explicitly starts playback.
- `PixelAnimation` defaults to 1000 ms duration, speed 1, looping, normal direction, and zero progress. Its SVG uses a `0 0 256 256` viewBox and crisp edges.
- `play` is idempotent while playing; `stop` pauses and resets progress and iteration. `seek` clamps to 0–100 and `setSpeed` clamps to at least 0.1; constructor duration and speed are accepted without those clamps.
- Unknown layer names return `null`. `destroy` pauses playback, removes the SVG, and clears the layer map.
- Compact animation data expands into percentage-sorted layer keyframes. Pixel snapshots accept direct arrays or `pixels-rle-v1` payloads; decoding caches the expanded array in the snapshot. Objects introduced at later keyframes stay hidden before their first appearance, and `hold` easing preserves the preceding snapshots between keyframes.
- The splash does not register teardown for its animation. Its update status state is not rendered.

## Side Effects

- Animation construction creates SVG nodes and optionally appends them to the supplied DOM container. Playback changes the SVG and uses browser timing; pausing cancels a pending animation frame.
- The splash calls the updater plugin's `check`. An available update is downloaded and installed, then the process plugin relaunches the application.
- With no update, or after a caught update-flow error, the splash waits 1000 ms and invokes the Tauri `hide_window` command. Caught errors are logged to the console.
- The splash applies transparent, margin-free, overflow-hidden body styling and renders a black viewport-sized surface. The timer page applies its own global body styling and invokes backend timer operations.

## Verification

From the repository root, run `pnpm check` and `pnpm build`. The package defines no test script.
