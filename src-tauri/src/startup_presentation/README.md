# Startup presentation internals

The parent [startup_presentation.rs](../startup_presentation.rs) owns native IPC,
window creation/cleanup, geometry, and the single deadline worker. These private
modules keep policy and platform mechanisms separate; they do not access Awake's
sleep inhibitor or timer state.

- [registry.rs](./registry.rs) owns typed requests, tuning validation, independent
  production/preview revisions, tokens, retained handles, deadlines, and policy
  tests. Identical duplicates keep their token/deadline; conflicting revisions
  fail. Cancellation and expiration prevent delayed replacement creation.
  Registry methods perform no native UI operations.
- [platform.rs](./platform.rs) shows compact windows on their owning UI thread
  while preserving focus: temporary nonfocusability on macOS, a scoped CBT hook
  on Windows, and persistent GTK mapping policy on Linux. Windows cleanup failure
  leaves a forwarding-only hook retained for retry before the next guarded show.

From the repository root:

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
```
 The PR check workflow builds the target-gated branches on all
three desktop platforms. Native desktop behavior still needs platform smoke
checks; registry tests do not prove focus or compositor behavior.
