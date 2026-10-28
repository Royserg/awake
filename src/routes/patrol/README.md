# Cursor patrol setup route

Native code owns `patrol-<setupId>-<surfaceIndex>` windows and binds each real caller to a display and measured native content rectangle. The same `/patrol` page renders everywhere; one caller receives the control panel. No label, native handle or display selection supplied by JavaScript authorizes an action.

The drawing area is untransformed. Pointer capture freezes its CSS rectangle; normalized samples map to native top-left desktop coordinates in Rust. macOS keeps points; Windows quantizes to integer pixels with half ties toward positive infinity. The SVG shows the ordered closed loop clipped to each screen. Point IDs/order survive dragging; removal does not reorder the remainder.

The pure client listens before its first fetch, ignores older snapshots, serializes updates, coalesces unsent previews and preserves the final release. Start freezes new interactions and drains existing captures, mutations and readiness reports across all overlays before native arming. A changed draft aborts Start; a fresh gesture is required. Review confirmation belongs to the exact rendered revision pair.

Esc/Cancel cancels the complete setup. The native global listener supplies Esc/takeover while running; webviews never move the cursor. Unmount only disposes local listeners and queued work. The narrow patrol capability grants event subscription, with no window, updater or process permissions.

Run `pnpm test:patrol` and `pnpm check`. Real pointer capture, mixed scales, focus and display transitions require the desktop scenarios in `docs/cursor-patrolling/validation.md`.
