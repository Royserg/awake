<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';

  type State = 'idle' | 'activated';
  let status = $state<State>('idle');

  onMount(async () => {
    try {
      const update = await check();
      if (update) {
        console.log(`Update available: ${update.version}`);
        await update.downloadAndInstall();
        await relaunch();
      }
    } catch (error) {
      console.error("Error checking for updates:", error);
    }
  });

  async function activate() {
    try {
      const response = await invoke("activate") as string;
      if (response === 'activated') {
        status = 'activated';
      }
      else {
        console.error("Unexpected response:", response);
      }
    } catch (error) {
      console.error("Error invoking 'activate':", error);
    }
  }


  async function deactivate() {
    try {
      const response = await invoke("deactivate") as string;
      if (response === 'deactivated') {
        status = 'idle';
      }
      else {
        console.error("Unexpected response:", response);
      }
    } catch (error) {
      console.error("Error invoking 'deactivate':", error);
    }
  }

  const toggle = () => {
    if (status === 'idle') {
      activate();
    } else {
      deactivate();
    }
  }
</script>

<main class="container">
  <h1>Awake</h1>
  <button onclick={toggle}>{status === 'idle' ? 'Activate' : 'Deactivate'}</button>
</main>

<style>

</style>
