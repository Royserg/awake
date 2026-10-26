# Tauri Native Package

## Overview

This package builds Awake's Rust desktop runtime and connects it to the
frontend through Tauri. [build.rs](./build.rs) delegates build integration to
`tauri_build::build()`. [Cargo.toml](./Cargo.toml) defines the `awake` package
and the `awake_lib` library, emitted as `staticlib`, `cdylib`, and `rlib`.

## Interface

- [Native runtime](./src/README.md): public `awake_lib::run()`, command handler
  contracts, sleep-prevention/timer state, and tray/window behavior. Command
  implementations are private Rust functions annotated for Tauri IPC.
- [Tauri configuration](./tauri.conf.json): product `awake`, identifier
  `com.awake.app`, and application version `0.3.3`. The configured `main`
  window is centered, 300 by 300, undecorated, and nonresizable.
- Development invokes `pnpm dev` and loads `http://localhost:1420`. Production
  invokes `pnpm build` and consumes frontend output from `../build`.
- [Default capability](./capabilities/default.json): applies to `main` and
  declares `core:default` and `opener:default` permissions.
- [Desktop capability](./capabilities/desktop.json): applies to `main` on
  macOS, Windows, and Linux and declares `updater:default` and `process:default`.

## Invariants

- Cargo and Tauri application versions currently match at `0.3.3`.
- The runtime depends on Tauri with `tray-icon` and `image-png` features,
  `nosleep`, and the opener, process, and updater plugins. Plugin initialization
  belongs to the runtime; configuration alone does not grant frontend access.
- Both capability files target `main`; neither explicitly lists the dynamically
  created `timer` window. Desktop updater and process access comes from the
  desktop capability, alongside the default capability's core and opener access.
- Content Security Policy is explicitly `null` in the application configuration.
- Runtime state and transitions follow the [native runtime contract](./src/README.md):
  startup activates sleep prevention without a timer; setting a timer activates
  and resets its deadline; cancellation preserves activation; successful
  deactivation clears both activation and timer metadata.
- Bundling is enabled for all targets, with platform icon paths and updater
  artifact generation configured.

## Side Effects

- Tauri build integration and configured lifecycle commands run build tooling
  and frontend development/production tasks. Packaging requests application
  bundles and updater artifacts.
- The native runtime controls user-idle display sleep prevention, changes tray
  presentation, creates or changes webview windows, and can exit the process.
- The updater configuration supplies a public verification key and the
  [GitHub release metadata endpoint](https://github.com/Royserg/awake/releases/latest/download/latest.json).
  Endpoint configuration and plugin initialization do not establish that an
  update request, download, or installation occurs.

## Verification

From the repository root:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
```

Frontend integration checks from the repository root are `pnpm check` and
`pnpm build`. The Rust test command validates library behavior; native tray and
window interactions also require running the desktop application.
