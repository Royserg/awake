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
| `hide_window` | None | Hides `main`; requests Accessory activation policy on macOS. |
| `show_splash` | None | Shows and focuses `main` when it exists. |
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
- Closing `timer` is allowed. Closing another window hides it and prevents its
  destruction, keeping the tray application available.

## Side Effects

- Native `NoSleep` start/stop operations control prevention of user-idle display
  sleep. Transition failures are returned as strings to command callers.
- Activation and timer commands request tray refreshes; refreshes update menu
  text and may replace the tray icon using embedded on/off PNG assets. Icon
  decoding failure panics.
- Setup refreshes the tray and starts the periodic timer/refresh worker.
- A left-button release on the tray toggles activation. A right-button press
  refreshes it. Toggle failures are printed to stderr. The timer menu opens the
  timer window; the quit menu exits the process. Debug builds also handle the
  splash menu action.
- The timer webview is titled `Set Timer`, sized 280 by 200, nonresizable,
  always on top, and initially focused. Window show/hide/focus/create/destroy
  errors are generally ignored by the handlers.
- Hiding or closing the main window requests macOS Accessory activation policy.
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

