<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type State = 'idle' | 'activated';
  let status = $state<State>('idle');

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
