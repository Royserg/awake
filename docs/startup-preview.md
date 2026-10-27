# Startup eye replay and tuning

## Browser preview

Run `pnpm preview:intro`, or open `/preview` on the Vite server at port 1420.
The preview preserves the existing 16×16 pixel eye, adds eased motion and pupil
appearance, a bounded pixel shock wave, a short hold, and a whole-scene fade.
Initial timing is 850 ms opening, 600 ms hold, and 300 ms disappearance.

- **Replay / R:** new simulated launch using the current tuning.
- **Pause / Space:** freeze/resume virtual time, without a resume jump.
- **Timeline:** pause and sample the current visual. It does not advance an
  update or rewind its phase. A completed intro can be sampled again.
- **Stop:** clear simulated promises, visual deadlines, and clock tasks.
- **Fine tune:** change timing, pixel size, wave, backdrop, and closed-eye idle.
  Invalid values show an error and retain the last valid visible tuning.
- **Reset / Copy tuning:** restore initial values or copy accepted candidates
  as JSON. Tuning does not persist or alter production settings.
- **Viewport / Expand stage:** inspect 1:1 CSS size or fit fixed HD, 4K, and
  ultrawide viewports. Expand uses browser fullscreen; Escape exits it.

The stage background is a simulated desktop, not a desktop capture. Browser
replay cannot establish real transparency, monitor placement, focus behavior,
native size, or compositor flashes.

## Update transition rehearsal

Select ordinary launch, slow check, update at 300 ms, update after intro, long
download, installation, or a download/install/restart failure. Advance simulated
operations with Download complete, Install complete, and Fail current step.
Hide wait leaves the simulated update active. Try again rechecks/downloads;
Restart only retries relaunch. Installation failure does not offer retry.

Windows handoff removes the old simulated view. Simulate restart creates a new
launch with the normal opening. No real check, download, installation, process
restart, sleep-state change, or timer operation occurs in this route.

## Native replay

Run `pnpm d`, right-click the tray, and choose **Intro Preview** (debug builds
only). Enable **Native desktop replay**. The controller continues simulating
updates while the native surface uses actual selected-monitor geometry and
transparent presentation. Pause/scrub/tuning/status and Stop use only debug
preview commands; no real update, relaunch, sleep-state, or timer action is sent.
The 30-second native lease is absolute, including pauses. Replay starts a fresh
lease. A completed/expired native surface remains ended until Replay.

Ordinary startup now uses the same renderer and native presentation service.
The hidden main webview checks for updates independently and remains alive when
presentation is hidden. A confirmed update cancels the overlay and shows a
compact closed eye, including during installation. Its Hide action leaves the
update running. Errors stay hidden after a user hide. Reduced motion uses a
250 ms static compact opening without wave, dimming, or idle travel.

macOS/Linux request relaunch after install; Windows may exit into its external
installer before JavaScript resumes. The new process plays the normal opening.
Do not use preview controls as evidence of an actual installed update.

## Progress and evidence

All implementation phases of the [design](./designs/cinematic-eye-startup.md)
are connected. The user approved the initial browser motion before native
integration: 850/600/300 ms, 15 CSS-pixel cells, wave delay 200 ms and opacity
0.25, center/edge dimming 0.55/0.30, idle cycle 2400 ms and lift 0.5 rows.
This approval does not substitute for platform desktop acceptance.

Verified locally on 2026-10-27:

- `pnpm check`: zero errors/warnings; pure test types pass.
- `pnpm test:startup`: 32 tests pass under Node 25.7.0, including native bridge
  races, cancellation barriers, fallback, preview isolation, installation DOM
  flush, and compatibility-show behavior. Node 22.17 was not separately executed.
- `pnpm build`: static root, `/intro`, and `/preview` assets build successfully.
- macOS `cargo test --locked --offline --lib`: ten tests pass (seven portable
  presentation-policy tests and three real sleep/timer regression tests).
- macOS `cargo check --locked --offline --release --lib`: passes, including
  the release handler set with no preview commands.
- The debug macOS `.app` builds. A normal launch exposed the native intro route
  and subsequently removed its surface while the updater owner stayed hidden.
- Native replay was exercised in a temporary debug build that also opens the
  preview controller from startup configuration. This test-only configuration
  lived outside the repository. The shipping configuration still opens only
  hidden `main`; normal debug access is through the tray menu.
- Actual native readiness/ended receipts, Pause/seek, the 30-second lease, early
  update handoff to active compact waiting, installation labeling, hidden
  installation failure, and delayed update discovery were checked through the
  native controller. It retained input focus during the compact handoff.
- The early handoff revealed Tauri's queued destroy/removal race. The fixed-label
  replacement now waits for manager removal within its original preparation
  deadline; the corrected native early transition reached active compact waiting.
- Existing browser checks preserved SVG object/pixel nodes, the 256-node wave
  pool, endpoint artwork, reduced motion, fixed 4K scaling, and browser controls.

A macOS application bundle was produced without updater artifact signing for
smoke testing. The standard updater-artifact build requires its existing private
signing key; signed delivery/install/relaunch was not exercised here.

Still pending: Windows and Linux native builds and desktop checks (including
Windows CBT cleanup, Linux X11/Wayland mapping), transparency/compositor flash
inspection, click-through/focus checks against another app, negative-origin and
mixed-DPI monitors, hidden downloads longer than five minutes, signed packaged
update delivery, Windows installer handoff, and actual updated-process relaunch.
Native preview receipts and unit checks do not prove these acceptance cases.

## Merge checks

Unused local/parameter checks are enabled for the frontend and pure test graph.
The canonical renderer lives in the shared library, with the old route exports
retained as aliases. Its commented-out usage block and unused helpers are removed.
Rust registry policy and platform show helpers live in separate private modules;
Clippy runs with warnings denied. The new PR workflow checks macOS, Windows,
and Linux without updater secrets or release publishing. Local macOS checks do
not establish that those new remote CI jobs have passed; run them before merge.
The existing native/platform and signed-updater acceptance limits above remain.
