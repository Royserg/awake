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
