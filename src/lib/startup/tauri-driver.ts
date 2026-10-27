import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import type { Clock, UpdateDriver } from './types.ts';
import { CHECK_TIMEOUT_MS } from './tuning.ts';

export const desktopClock: Clock = {
  now: () => performance.now(),
  schedule: (delay, callback) => { const id = setTimeout(callback, delay); return () => clearTimeout(id); },
};

/** Sole production updater adapter. Preview code never imports this module. */
export const desktopUpdates: UpdateDriver = {
  async check() {
    const update = await check({ timeout: CHECK_TIMEOUT_MS });
    if (!update) return null;
    let closed = false;
    return {
      version: update.version,
      async download(progress) {
        let downloaded = 0, total: number | undefined;
        await update.download(event => {
          if (event.event === 'Started') { downloaded = 0; total = event.data.contentLength; }
          if (event.event === 'Progress') downloaded += event.data.chunkLength;
          progress({ downloaded, total });
        });
      },
      install: () => update.install({ restartAfterInstall: true }),
      relaunch,
      async close() { if (!closed) { closed = true; await update.close(); } },
    };
  },
};
