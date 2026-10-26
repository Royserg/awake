# Awake

Cross-platform (tested on Mac) app that prevents system from sleeping.

![app toggling](./docs/awake-toggling.gif)

## Install

Download the latest release from [GitHub Releases](https://github.com/Royserg/awake/releases/latest).

On macOS, if you see a message that the app is "broken" or can't be opened, run this command to remove it from quarantine:

```bash
xattr -cr /Applications/Awake.app
```

## Features

- Toggle sleep prevention with a single click
- Visual indicator in system tray
- Timer for keeping system awake for a period of time

## Usage

- Left-click the tray icon to toggle sleep prevention
- Right-click to see the awake status, set or cancel a timer, or quit
- When a timer finishes, sleep prevention turns off. Left-click to turn it on again without a timer.

## Development

Requires Node.js 22.17 or newer (CI uses Node.js 24), pnpm, and a current stable Rust toolchain.
Frontend configuration lives in `vite.config.js` using SvelteKit 3.

Built with:

- [Tauri](https://tauri.app/)
- Rust

## System Overview

Awake is a desktop tray application with a Rust backend and a statically built
SvelteKit frontend. Rust owns the sleep inhibitor, activation state, timer,
tray, and webview windows. The frontend supplies the startup splash, update
flow, and timer controls through Tauri commands and plugins.

The root [index.html](./index.html) is a standalone landing page. The desktop
webview document comes from [src/app.html](./src/app.html), and its generated
production assets are written to `build/`.

## Local Interface & Invariants

### Interface

| File | Contract |
| --- | --- |
| [package.json](./package.json) | Defines the `awake` ESM package, development/build/check scripts, and frontend dependencies. `pnpm d` runs Tauri development; `pnpm tauri` exposes the Tauri CLI. |
| [vite.config.js](./vite.config.js) | Exports Vite configuration using SvelteKit, `vitePreprocess`, and the static adapter. |
| [tsconfig.json](./tsconfig.json) | Extends `$app/tsconfig` with strict TypeScript and JavaScript checking and bundler module resolution. |
| [index.html](./index.html) | Provides the standalone landing page and animated tiled background. Its Download link currently targets `#`. |
| [justfile](./justfile) | Contains development and icon shortcuts, plus legacy build/database recipes. `b` refers to an absent `t:build` package script; SQLx recipes require tooling and configuration not established by the current manifests. |
| [Release workflow](./.github/workflows/build-prod.yml) | A push to `prod` creates a draft release, builds macOS ARM/Intel, Linux, and Windows artifacts, then publishes after the build job succeeds. Release names and tags come from `package.json`. |
| [Pages workflow](./.github/workflows/static.yml) | A push to `prod` or manual dispatch requests a GitHub Pages deployment, with `./index.html` configured as the upload path. |

### Invariants

- The Vite development port is fixed at 1420 with `strictPort` enabled, matching
  Tauri's configured development URL. `TAURI_DEV_HOST` optionally supplies the
  host; remote-host HMR uses WebSocket port 1421.
- Vite preserves terminal output and excludes `src-tauri` from its file watcher.
- Use the package scripts and verification commands below for current checks;
  legacy `justfile` entries do not establish working build or database targets.
- Keep the standalone landing page separate from the SvelteKit application
  document and routes.

## Subdirectories & System Boundaries

| Boundary | Responsibility |
| --- | --- |
| [Frontend source](./src/README.md) | HTML document shell and the frontend route tree. |
| [Frontend routes](./src/routes/README.md) | Static rendering policy, splash animation, updater flow, and public `PixelAnimation` interfaces. |
| [Timer route](./src/routes/timer/README.md) | Minute-based selection and cancellation controls that call the native timer commands. |
| [Tauri package](./src-tauri/README.md) | Rust build integration, application configuration, window capabilities, bundling, and updater configuration. |
| [Native runtime](./src-tauri/src/README.md) | `awake_lib::run()`, registered IPC handlers, sleep/timer state transitions, tray events, and windows. |

`docs/`, `static/`, and the native icon directories hold supporting assets.
Generated output, dependencies, and asset-only directories are outside this
contract tree.

## Global Invariants

- Rust is the source of truth for activation and timer state. Startup enables
  `PreventUserIdleDisplaySleep` without a timer; the frontend does not maintain
  an independent sleep inhibitor.
- Setting a timer enables Awake if needed and replaces its deadline. Cancelling
  a timer leaves Awake enabled. Successful manual deactivation and expiry clear
  activation and timer metadata; subsequent activation starts without a timer.
- The timer UI selects minutes and invokes `set_timer_duration` with
  `{ durationMs }` in milliseconds. Tauri maps that argument to Rust's
  `duration_ms`; remaining time is a millisecond number or `null`.
- The background worker queues one-second timer ticks onto the main thread.
  Release the application-state lock before refreshing tray UI, and do not hold
  a worker lock while waiting for main-thread UI work.
- Tray left-button release toggles Awake. The menu is disabled on left-click
  and opens on right-click through Tauri. Preserve the tray dependency's macOS
  click-forwarding fix when changing dependency versions.
- Routes are prerendered with server-side rendering disabled. Preserve
  `%sveltekit.head%` and `%sveltekit.body%` in the document shell.
- The native `timer` window loads `/timer`. Closing it is allowed; closing the
  main window hides it so the tray application remains available.
- Capability files currently target `main`. Core/opener permissions and desktop
  updater/process permissions are declared separately; the dynamically created
  timer window is not explicitly listed in either file.
- The application version is currently `0.3.3` in `package.json`, `Cargo.toml`,
  and `tauri.conf.json`. Keep release metadata consistent across these files.

## Global Side Effects

- Native activation and deactivation change the operating system's user-idle
  display sleep prevention. The process also updates tray icons/menu text and
  creates, focuses, hides, or destroys webview windows.
- The startup splash checks the configured GitHub updater endpoint. When an
  update is available, it downloads and installs it, then relaunches the app.
  With no update or after a caught update error, it hides the splash after a
  one-second delay.
- Frontend animation and page styling mutate the webview DOM. The standalone
  landing page separately creates its tiled background and updates hover
  styling.
- Development/check commands generate framework metadata and start tooling;
  production builds write static frontend assets. Native packaging produces
  platform bundles and requests signed updater artifacts.
- The release workflow creates and publishes GitHub releases and uses configured
  signing secrets for updater artifacts. The separate Pages workflow uploads
  its configured artifact and requests deployment; its presence does not prove
  a successful deployment or working hosted assets.
- Activation timestamps and timers are in memory for the current process;
  there is no application-state persistence layer in this implementation.

## System Verification

From the repository root:

```sh
pnpm check
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
```

The frontend package defines no test script. The current native regression
tests run only on macOS and exercise the real sleep inhibitor without launching
the UI or updater. They cover expiry/reactivation, manual toggles, timer
replacement, and cancellation.

For desktop integration, run `pnpm d`. Verify left-click off/on, the right-click
status/timer menu, timer cancellation, and reactivation after expiry. Build and
unit checks do not simulate native tray clicks or prove updater delivery.
