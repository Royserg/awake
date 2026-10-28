# Shared frontend modules

[Startup animation and replay](./startup/README.md) owns pure launch policy,
elapsed-time motion, and the shared Svelte visual. Routes mount these modules;
the hidden root route owns real updater work, the passive intro route renders
native-owned surfaces, and the preview injects only simulated updates.

Shared modules must not start sleep prevention or native updates on import.

## Cursor patrol state and transport

`patrol-state.ts` has no Svelte/Tauri imports. It validates closed wire records, explicit nulls, canonical decimal u64 strings, finite geometry and snapshot invariants. IDs use BigInt for comparison/allocation; bounded pause milliseconds use numbers. Snapshots render monotonically while receipts retain their own canonical mutation result.

One update is in flight per overlay. Captured samples keep their source mapping and release coordinates; unsent previews coalesce at 30 Hz. Finalization drains the existing drag and queue before reporting readiness and acknowledging Start. Concurrent target edits and uncertain move results fail closed. Review confirmation freezes the displayed route/layout pair before queueing and never refreshes consent on a conflict. Failure latches belong to their source. Disposal only removes local listeners and queued work.

`patrol.ts` adapts six native commands and the targeted `patrol-state-changed` event. Subscribe before fetching; native caller identity supplies the display mapping. See `patrol-state.test.ts` and `pnpm test:patrol`.
