# Passive startup surface

`/intro?runId=…` is embedded only in native-owned `startup-intro` or debug
`startup-preview-surface` windows. The URL is an identifier, not authority:
`intro_config` validates the caller and current token and returns mode, tuning,
reduced-motion policy, and whether the native preview namespace applies.

The route prepares the shared `StartupVisual`, flushes Svelte DOM changes, then
acknowledges readiness. Native code shows the window and returns elapsed time.
The cinematic page stays transparent, marginless, and noninteractive; compact
preview states have a black background and simulated status labels. Real update
waiting belongs to the root main webview, not this surface.

One animation frame drives elapsed-time sampling. Completion finishes only its
own token. Debug controls support Pause/Resume/Seek/tuning/status and time samples
at most every 100 ms. Static reduced-motion waits and errors need no idle loop.
Teardown cancels frames, ResizeObserver, and event listeners. Independent native
deadlines remove abandoned surfaces even if JavaScript stops responding.

This route imports neither updater/process plugins nor activation/timer commands.
Run `pnpm check`, `pnpm test:startup`, and `pnpm build`; use debug native replay
for actual placement, transparency, focus, and cleanup acceptance.
