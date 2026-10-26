## README.md

- **Coverage gap — corrected:** The blueprint omitted the workflows' external
  release/deployment side effects. The [release workflow](./.github/workflows/build-prod.yml)
  triggers on `prod`, derives release metadata from `package.json`, creates a
  draft, builds four platform/architecture targets, and publishes after success.
  The [Pages workflow](./.github/workflows/static.yml) also triggers on `prod`
  or manual dispatch and configures `./index.html` as its upload path. Both
  workflow interfaces and their side effects are now documented in
  [README.md](./README.md). Workflow execution was not performed by this review.

## src/README.md

No drift detected.

## src/routes/README.md

- **Coverage gap — corrected:** The public completion event and callback
  timing were missing. [eye-splashscreen.ts](./src/routes/eye-splashscreen.ts),
  lines 391–441, calls `onComplete` on wraps and final completion, calls
  `onFrame` on playback ticks, and on final completion calls `onFinished` and
  dispatches `animation-finished` with `{ progress, iteration }`. These are now
  in the [route interface contract](./src/routes/README.md).
- **Coverage gap — corrected:** Snapshot encoding and keyframe visibility
  rules were omitted. The same implementation expands compact data into sorted
  keyframes (lines 193–243), filters objects by their first appearance and
  handles `hold` snapshots (lines 469–539), and caches decoded `pixels-rle-v1`
  data back into snapshots (lines 763–807). The route invariants now describe
  these behaviors.

## src/routes/timer/README.md

No drift detected.

## src-tauri/README.md

No drift detected.

## src-tauri/src/README.md

- **Coverage gap — corrected:** The user-visible tray status/timer labels and
  their time granularity were omitted. [lib.rs](./src-tauri/src/lib.rs), lines
  162–245, formats elapsed whole minutes, inactive status, timer hours/minutes,
  and the `Set Timer`/`Timer` fallbacks. Remaining time is truncated to whole
  seconds before rounding to minutes. These rules are now in the
  [native runtime contract](./src-tauri/src/README.md).
- **Coverage gap — corrected:** The contract did not distinguish menu-label
  and icon update failures. In `lib.rs`, lines 247–302, menu-text caches change
  before ignored `set_text` results, so unchanged labels are not retried;
  the icon cache changes only after a successful icon update. Both retry
  behaviors are now documented. No application code was changed.
