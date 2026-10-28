import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { decodeReply, PatrolClient } from './patrol-state.ts';
import type { Transport } from './patrol-state.ts';
async function call(command: string, args: Record<string, unknown> = {}) { return decodeReply(await invoke<unknown>(command, args)); }
export function createPatrolClient(changed: () => void): PatrolClient {
    const transport: Transport = {
        get: () => call('get_patrol_setup'),
        update: input => call('update_patrol_setup', { ...input }),
        start: input => call('start_patrol', { ...input }),
        cancel: setupId => call('cancel_patrol_setup', { setupId }),
        permissions: setupId => call('request_patrol_permissions', { setupId }),
        listen: changed => listen('patrol-state-changed', changed)
    };
    return new PatrolClient(transport, changed);
}
