# Startup animation and preview contract

## Interface

- `types.ts` defines tuning, frame/snapshot data, clock, update session, and
  presentation-driver interfaces. Controller snapshots are immutable; `busy`
  remains true until an update attempt's settled resource is closed.
- `tuning.ts` provides frozen defaults and validation. Total cinematic duration
  is 1500–2000 ms; the initial target is 1750 ms. Artwork cells use CSS pixels.
- `frame.ts` samples eased opening, bounded pixel wave, hold/exit, reduced-motion
  static poses, and closed-eye idle from elapsed time. It has no DOM or timers.
- `pixel-eye.ts` owns the single canonical artwork, sampler, and SVG renderer.
  The old route module forwards its public exports for compatibility.
- `StartupVisual.svelte` renders those frames with the canonical eye renderer
  and a fixed pool of 256 wave cells. The caller supplies elapsed time; the
  visual does not start a second frame loop or import native APIs.
- `controller.ts` starts a check before presentation, owns update sequencing,
  and rejects stale visual completions by revision. Its public operations are
  `start`, `subscribe`, `hideWait`, `retryDownload`, `retryRestart`,
  `stopPresentation`, `showStaticIntro`, and `dispose`. An optional `beforeInstall` hook flushes the installing state before the installer runs; visual-hook failure cannot stop installation.
- `preview-driver.ts` supplies `VirtualClock`, `SimulatedUpdates`,
  `BrowserPresentation`, `StartupPreview`, and the `PREVIEW_SCENARIOS` index /
  `PreviewScenario` type. They contain no updater,
  process, activation, or timer imports. Native replay receives an injected
  `NativePresentation` and uses only its preview namespace.

## Invariants and lifetime

Download resolution precedes installation; install resolution precedes relaunch.
Visual failure cannot bypass checking or strand an update. Hiding presentation
does not cancel the update owner. Download/restart recovery is explicit and
installation is not retried. Actual component disposal detaches observers;
an in-flight update resource closes after settlement, never during install.

The presentation driver owns visual completion/deadlines. Browser presentation
uses the same virtual clock as the simulated update. Seeking resamples only
the current/last visual and reschedules its visual completion; it does not
advance the clock, dispatch fake update events, or rewind update state.
This allows inspection of a completed opening without creating another launch.
Replay creates a fresh controller and simulated session.

Preview Stop disposes the controller, simulated promises, visual deadlines, and
clock tasks. The route cancels its browser animation frame. Reduced motion
disables idle travel, wave, and dimming and uses a 250 ms static open-eye intro.

## Native integration boundary

`native-presentation.ts` is a pure injected IPC coordinator. It registers events
before commands, initializes revisions from the native context, accepts only
monotonic receipts, queues preview controls until readiness, and uses cancellation
barriers while a create reply is pending. A hide revision remains a tombstone,
so a request at that revision cannot reopen presentation. Late token-only cleanup cannot hide a
newer wait. A failed cinematic may fall back to a 250 ms compact static intro.
`tauri-port.ts` adapts core invoke/event APIs; it imports no updater or process API.

`tauri-driver.ts` alone imports real updater/process plugins. Each update attempt
owns one resource, emits byte progress, splits download/install, and closes the
resource exactly once after settlement. The root route supplies the production
adapter; preview injects fake updates even when native surfaces are enabled.
Native context and command guards enforce the window-label boundary separately.

## Verification

Run `pnpm check`, `pnpm test:startup`, and `pnpm build` at the repository root.
The tests cover preserved artwork endpoints, easing/wave bounds, update order,
early/late discovery, hidden failures, retry isolation, clock controls, and
simulated scenarios without native IPC. See the
[replay guide](../../../docs/startup-preview.md) for browser and native limits.
