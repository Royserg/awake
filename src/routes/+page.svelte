<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { PixelAnimation } from "./eye-splashscreen";

  let updateStatus = $state<string>("");
  let isUpdating = $state(false);

  let animationContainer: HTMLElement;

  onMount(async () => {
    const splashAnimation = new PixelAnimation({
      container: animationContainer,
      duration: 1000,
      repeat: "once",
    });
    splashAnimation.play();

    // Check for updates
    try {
      const update = await check();
      if (update) {
        isUpdating = true;
        updateStatus = `Updating to v${update.version}...`;
        await update.downloadAndInstall();
        await relaunch();
      } else {
        await hideSplash();
      }
    } catch (error) {
      console.error("Error checking for updates:", error);
      await hideSplash();
    }
  });

  async function hideSplash() {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    await invoke("hide_window");
  }
</script>

<main class="splash">
  <div class="container" bind:this={animationContainer}></div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
  }

  .container {
    width: 80%;
    height: 80%;
  }

  .splash {
    width: 100vw;
    height: 100vh;
    background: #000;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    font-family: "Courier New", monospace;
    color: white;
  }
</style>
