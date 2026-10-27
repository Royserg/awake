import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { NativePort } from './native-presentation.ts';

export { isTauri };
export const tauriPort: NativePort = {
  invoke,
  listen<T>(event: string, callback: (payload: T) => void) { return listen<T>(event, event => callback(event.payload)); },
};
