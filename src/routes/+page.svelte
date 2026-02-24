<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  let updateStatus = $state<string>("");
  let isUpdating = $state(false);
  let animationPhase = $state<"closed" | "opening" | "open">("closed");

  onMount(async () => {
    // Phase 1: Start opening after brief moment
    setTimeout(() => {
      animationPhase = "opening";
    }, 300);

    // Phase 2: Fully open
    setTimeout(() => {
      animationPhase = "open";
    }, 800);

    // Check for updates
    try {
      // update logic:
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
    await new Promise((resolve) => setTimeout(resolve, 800));
    await invoke("hide_window");
  }
</script>

<main class="splash">
  <div class="eye-container">
    <div
      class="pixel-eye"
      class:opening={animationPhase === "opening"}
      class:open={animationPhase === "open"}
    >
      <!-- Eye shape made of pixel rows -->
      <div class="row r1"></div>

      <div class="row r2">
        <div class="pixel"></div>
        <div class="gap" style="width: calc(var(--p) * 4)"></div>
        <div class="pixel"></div>
      </div>

      <div class="row r3">
        <div class="pixel"></div>
        <div class="gap" style="width: calc(var(--p) * 6)"></div>
        <div class="pixel"></div>
      </div>

      <div class="row r4">
        <div class="pixel"></div>
        <div class="gap" style="width: calc(var(--p) * 8)"></div>
        <div class="pixel"></div>
      </div>

      <div class="row r5">
        <div class="pixel"></div>
        <div class="gap" style="width: calc(var(--p) * 6)"></div>
        <div class="pixel"></div>
      </div>

      <div class="row r6">
        <div class="pixel"></div>
        <div class="gap" style="width: calc(var(--p) * 4)"></div>
        <div class="pixel"></div>
      </div>

      <div class="row r7"></div>

      <!-- Pupil appears when open -->
      <div class="pupil"></div>
    </div>
  </div>

  {#if updateStatus}
    <div class="update-status">{updateStatus}</div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
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
  }

  .eye-container {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :root {
    --p: 8px;
  }

  /* Pixel eye container */
  .pixel-eye {
    position: relative;
    width: calc(var(--p) * 10);
    height: calc(var(--p) * 7);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    transform: scaleY(0.15);
    opacity: 0.8;
    transition: transform 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .pixel-eye.opening {
    transform: scaleY(0.6);
    opacity: 1;
  }

  .pixel-eye.open {
    transform: scaleY(1);
    opacity: 1;
  }

  /* Each row is a pixel row of the eye */
  .row {
    display: flex;
    height: var(--p);
    flex-shrink: 0;
  }

  /* Edge rows - solid (top and bottom of outline) */
  .r1,
  .r7 {
    background: white;
  }

  .r1 {
    width: calc(var(--p) * 4);
  }

  .r7 {
    width: calc(var(--p) * 4);
  }

  /* Middle rows - outline only using 3-div structure */
  .r2,
  .r3,
  .r4,
  .r5,
  .r6 {
    background: transparent;
  }

  .pixel {
    width: var(--p);
    height: var(--p);
    background: white;
  }

  .gap {
    height: var(--p);
    background: transparent;
  }

  /* Pupil - 2x2 white, only visible when open */
  .pupil {
    position: absolute;
    width: var(--p);
    height: var(--p);
    background: white;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    opacity: 0;
    transition: opacity 0.3s ease 0.3s;
  }

  .pixel-eye.open .pupil {
    opacity: 1;
  }

  .update-status {
    position: absolute;
    bottom: 24px;
    color: white;
    font-size: 10px;
    letter-spacing: 1px;
    text-transform: uppercase;
  }
</style>
