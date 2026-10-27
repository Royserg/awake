# Frontend source blueprint

## Overview

This subtree contains Awake's SvelteKit document shell and frontend routes. Tauri hosts the statically generated UI; the route components delegate window, timer, update, and process operations to native commands or plugins.

## Interface

- [`app.html`](./app.html) defines the English-language HTML document, `Awake` title, UTF-8 encoding, viewport metadata, and favicon asset reference. It provides `%sveltekit.head%` and `%sveltekit.body%` injection points.
- [Routes and animation contract](./routes/README.md): defines static rendering, the animated splash and updater flow, the public `PixelAnimation` controller and data types, and the timer child route.
- [Passive intro route](./routes/intro/README.md): renders native-owned run tokens without updater or sleep-state access.
- [Shared startup modules](./lib/startup/README.md): shared animation/controller code, injected presentation bridge, simulated preview drivers, and the production updater adapter.

## Invariants

- Keep the SvelteKit template placeholders intact. The body wraps injected content in a `display: contents` element and declares `data-sveltekit-preload-data="hover"`.
- The route layout enables prerendering and disables server-side rendering for Tauri's static frontend.
- The document shell owns page metadata; routes own UI behavior and their native API calls. The timer UI uses minutes and sends milliseconds over IPC.

## Side Effects

- The browser renders injected route content, loads the favicon, and enables hover-based data preloading through SvelteKit.
- Child routes mutate the DOM for animation, query and mutate native timer state, hide the splash window, and check for, install, and relaunch application updates through Tauri APIs.

## Verification

From the repository root, run `pnpm check`, `pnpm test:startup`, and `pnpm build`.
