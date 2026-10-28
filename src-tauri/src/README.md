# Native Awake Runtime

## Overview

This Rust module owns Awake's native sleep-prevention state, timer behavior,
tray interactions, and webview windows. The binary entry point in
[main.rs](./main.rs) calls the public `awake_lib::run()` function in
[lib.rs](./lib.rs). Rust helper types and command implementations are private;
the Tauri command boundary exposes their application behavior to the frontend.

## Interface

`pub fn run()` initializes the Tauri application with the process, updater, and
opener plugins and installs managed application state.

The registered Tauri IPC handlers in `lib.rs` have the following contracts.
`AppHandle` and `State<Mutex<AppState>>` parameters are supplied by Tauri.

| Handler | Explicit Rust input | Result and behavior |
| --- | --- | --- |
| `hide_window` | None | Ends the current compact presentation and hides `main`; emits the user-hide event. |
| `show_splash` | None | Requests a short static intro from the main owner; does not recheck updates or interrupt an update. |
| `activate` | None | Returns `"activated"` after a successful activation; otherwise a string error. |
| `deactivate` | None | Returns `"deactivated"` after a successful deactivation; otherwise a string error. |
| `set_timer_duration` | `duration_ms: u64` | Activates Awake, replaces the timer, and destroys an existing `timer` window; returns success or a string error. |
| `cancel_timer` | None | Clears the timer while retaining activation. |
| `get_timer_remaining_ms` | None | Returns remaining whole milliseconds, or no value when timer metadata is absent or the deadline has passed. |
| `open_timer_window` | None | Shows/focuses `timer`, or creates it at `/timer`. |

The frontend sends `set_timer_duration` as `{ durationMs: number }`; Tauri maps
that camelCase argument to the Rust `duration_ms` parameter. Optional remaining
time is serialized as a number or `null` for JavaScript callers.

`AppState`, `StateSnapshot`, tray display state, and their helper functions are
internal implementation details. `AppState` owns the `NoSleep` instance,
activation flag, monotonic activation timestamp, and optional timer duration.

## Invariants

- Initialization starts `PreventUserIdleDisplaySleep` immediately, with an
  activation timestamp and no timer. Creation or initial start failures panic.
- Activation starts sleep prevention only when inactive. A successful inactive
  to active transition records a new timestamp and clears the timer; repeated
  activation while already active preserves both.
- Successful deactivation clears the active flag, timestamp, and timer. If
  stopping sleep prevention fails, the error returns before those changes.
- Setting a timer activates Awake if necessary, then replaces its timestamp and
  duration. Zero duration is accepted. Cancelling clears only the duration.
- Remaining time uses `Instant` elapsed time and truncates to milliseconds. A
  passed deadline returns no remaining value; querying does not deactivate
  Awake. Internal expiry deactivates an active timer when elapsed time reaches
  or exceeds its duration.
- Shared application state is protected by `Mutex<AppState>`. Inspected command
  and tray paths require successful lock acquisition and can panic on poison.
- The background worker sleeps for one second between queued refresh tasks.
  Timer expiry runs inside a `run_on_main_thread` closure, which releases the
  application-state lock before refreshing the tray. Timer expiration is
  therefore processed on a periodic main-thread task rather than by the
  remaining-time query.
- Tray refresh copies an application-state snapshot before locking display
  state. The cached activation flag changes only after the icon update succeeds.
- The disabled status menu item shows `○ Off` when inactive and `● Awake (m)`
  or `● Awake (h:mm)` for elapsed whole minutes when active. Without timer
  metadata the timer item reads `Set Timer`; otherwise it shows `Timer: Xm`,
  `Timer: Xh`, or `Timer: Xh Ym`. Formatting first truncates remaining time to
  whole seconds, then rounds those seconds up to minutes; zero displays `Timer`.
- Menu labels are set only when their computed text changes. Their caches are
  updated before `set_text`, whose error is ignored, so an identical label is
  not retried until its computed text changes. Failed icon updates leave the
  cached activation flag unchanged and can be retried on a later refresh.
- Closing `main` hides it and preserves the updater owner. Closing the timer or
  preview controller is allowed; preview close ends its passive surface. Transient
  overlay destruction is allowed. Closing/tray events never mutate update state.

## Side Effects

- Native `NoSleep` start/stop operations control prevention of user-idle display
  sleep. Transition failures are returned as strings to command callers.
- Activation and timer commands request tray refreshes; refreshes update menu
  text and may replace the tray icon using embedded on/off PNG assets. Icon
  decoding failure panics.
- Setup refreshes the tray and starts the periodic timer/refresh worker.
- A left-button release on the tray toggles activation. A right-button press
  refreshes it. Toggle failures are printed to stderr. The timer menu opens the
  timer window; the quit menu exits the process. Debug builds also provide the
  Intro Preview menu action. Any tray interaction dismisses the cinematic overlay.
- The timer webview is titled `Set Timer`, sized 280 by 200, nonresizable,
  always on top, and initially focused. Window show/hide/focus/create/destroy
  errors are generally ignored by the handlers.
- macOS Accessory activation policy is established during setup.
  Windows release builds suppress the additional console window through the
  binary entry-point attribute.

## Verification

From the repository root:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
```

The current regression tests are enabled only on macOS and exercise the real
sleep inhibitor without launching the UI or updater. They cover repeated timer
expiry/reactivation, manual toggles, timer replacement, and cancellation. A test
run on another platform does not execute those macOS-only cases. Native tray
clicks and window behavior also require a desktop smoke check.


## Native startup presentation

[startup_presentation.rs](./startup_presentation.rs) owns IPC and native UI
operations, with no `AppState`/NoSleep reference. Its private
[registry and platform modules](./startup_presentation/README.md) separate
request/lifetime policy from target-specific compact-show mechanisms. One managed condition-variable worker
invalidates expired runs before queuing UI cleanup. Preparation and production
visible deadlines are two seconds each; debug surfaces have a 30-second absolute
lease. Tauri queues destruction, so a replacement fixed-label preview waits for
manager removal; the same worker retries preparation only while its token and
original deadline remain valid. Failed cleanup retains actual handles for retry.

Production owner `main` calls `startup_context`, `start_intro`, `show_compact`,
`hide_compact`, and `finish_intro`. Passive surfaces call `intro_config`, then
`intro_ready`/`finish_intro` for their matching run. Debug-only handlers are
`preview_context`, `preview_present`, `preview_ready`, `preview_control`,
`preview_sample`, and `preview_finish`; exact preview labels cannot access Awake
state, real updater/process grants, or production presentation commands.
Existing application commands accept only `main`/`timer` callers.

Each namespace retains its highest revision, typed request, decimal-string token,
and status. Identical duplicates preserve token/deadline; conflicting or stale
requests fail. Owners can send a higher revision with a null token to cancel
pending creation; passive surfaces can finish only their own token. Late token
cleanup cannot invalidate a newer run. Context commands return the existing
revision so a reloaded controller does not restart its sequence at zero.

Cinematic geometry uses cached launch-monitor physical bounds (cursor monitor,
then primary), transparent background, no decorations/shadow/taskbar presence,
nonfocusability, click-through, and always-on-top. Resize/scale/close invalidates
that run. `main` remains a compact updater owner, kept alive while hidden.

Compact show preserves focus through target-specific handling: macOS temporarily
sets nonfocusable, Windows uses a scoped UI-thread CBT hook around normal Tauri
show with forwarding-only cleanup retries, and Linux retains GTK focus-on-map
false through asynchronous mapping. GTK accept-focus stays true. Platform-native
acceptance beyond the local macOS smoke checks remains pending.

## Native cursor patrol coordination

`patrol/mod.rs` adds `get_patrol_setup`, `update_patrol_setup`, `start_patrol`, `cancel_patrol_setup`, `stop_patrol`, and `request_patrol_permissions` to both handler lists. Flat request bodies are decoded inside the handlers, so malformed requests receive typed errors. Only a registered current overlay can act; the controller additionally checks setup/layout revisions, control role, action identity and run generation.

The main-thread coordinator owns window, Awake and timer effects. Off cancels movement before the fallible inhibitor release, deferring that release until an in-flight synchronous native call returns and acknowledges stop. Timer changes share that ordering and publish actual monotonic deadlines. Native callbacks never wait for AppState or UI work. The worker checks 100 ms observer/availability leases during travel and pauses, immediately before each native position call, and checks the current Awake deadline independently of UI refresh.

Registered-overlay close/reload cancels setup; programmatic destruction is tracked separately. Cleanup failures retain a closing state instead of claiming success. Exit and process-plugin restart allow up to two seconds of cleanup. Tauri's reserved restart exit cannot be prevented (including installer-owned restarts), so that fallback synchronously invalidates native movement before process exit. Startup shutdown and its debug-only preview boundary remain intact. See `patrol/README.md` and the native validation record.
