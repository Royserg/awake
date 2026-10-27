# Timer route

## Overview

The timer window lets the user choose an Awake duration or cancel an existing timer. Its Svelte component delegates timer state to the Tauri backend.

## Interface

- [`+page.svelte`](./+page.svelte) is the `/timer` page; it exports no component props or named public symbols.
- On mount, `get_timer_remaining_ms` returns a remaining duration in milliseconds or `null`. A non-null value selects the running view and is rounded up to whole minutes for display.
- Start invokes `set_timer_duration` with the exact JavaScript payload `{ durationMs: minutes * 60 * 1000 }`. The UI selects minutes; IPC sends milliseconds.
- Cancel invokes `cancel_timer`, then selects the idle view. Cancellation leaves Awake enabled; timer and Awake behavior belong to the backend.

## Invariants

- The idle selection defaults to 30 minutes. Its slider allows 5–120 minutes in 5-minute steps.
- Idle mode shows the slider and Start; running mode shows Cancel timer. Durations display as minutes, whole hours, or hours plus remaining minutes.
- Initial timer state is queried once. This component has no countdown interval or event subscription.
- Starting awaits the backend without setting `timerRunning` locally. Cancellation changes the local state only after the backend call succeeds.

## Side Effects

- Mounting queries the Tauri backend; buttons invoke timer mutations. Mount-query errors are logged to the console, while button-call errors are not caught locally.
- The component applies global body styling with a dark background, zero margins, and hidden overflow, and renders a viewport-sized layout.

## Verification

From the repository root, run `pnpm check` and `pnpm build`. The package defines no test script.
