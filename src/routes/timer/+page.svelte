<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  // Slider value in minutes (5 to 120)
  let timerRunning = $state(false);
  let minutes = $state(30);
  let currentTimer = $state<number | null>(null);

  onMount(async () => {
    try {
      currentTimer = await invoke<number | null>("get_timer_remaining_ms");
      if (currentTimer !== null) {
        timerRunning = true;
        minutes = Math.ceil(currentTimer / 60000);
      }
    } catch (e) {
      console.error(e);
    }
  });

  function formatTime(mins: number): string {
    if (mins < 60) {
      return `${mins} min`;
    }
    const hours = Math.floor(mins / 60);
    const remainingMins = mins % 60;
    if (remainingMins === 0) {
      return `${hours}h`;
    }
    return `${hours}h ${remainingMins}m`;
  }

  async function startTimer() {
    const durationMs = minutes * 60 * 1000;
    await invoke("set_timer_duration", { durationMs });
  }

  async function cancelTimer() {
    await invoke("cancel_timer");
    timerRunning = false;
  }
</script>

<main>
  <div class="container">
    <div class="time-display">{formatTime(minutes)}</div>

    {#if !timerRunning}
      <input
        type="range"
        min="5"
        max="120"
        step="5"
        bind:value={minutes}
        class="slider"
      />

      <div class="labels">
        <span>5m</span>
        <span>2h</span>
      </div>
    {/if}

    <div class="buttons">
      {#if timerRunning}
        <button class="btn cancel" onclick={cancelTimer}>Cancel timer</button>
      {:else}
        <button class="btn start" onclick={startTimer}>Start</button>
      {/if}
    </div>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #1a1a1a;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
      sans-serif;
  }

  main {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px;
    width: 100%;
    box-sizing: border-box;
  }

  .time-display {
    font-size: 32px;
    font-weight: 600;
    color: white;
    letter-spacing: 1px;
  }

  .slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    max-width: 240px;
    height: 6px;
    background: #333;
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    background: white;
    border-radius: 50%;
    cursor: pointer;
    transition: transform 0.1s ease;
  }

  .slider::-webkit-slider-thumb:hover {
    transform: scale(1.1);
  }

  .slider::-moz-range-thumb {
    width: 20px;
    height: 20px;
    background: white;
    border-radius: 50%;
    cursor: pointer;
    border: none;
  }

  .labels {
    display: flex;
    justify-content: space-between;
    width: 100%;
    max-width: 240px;
    color: #666;
    font-size: 11px;
  }

  .buttons {
    display: flex;
    gap: 12px;
    margin-top: 8px;
  }

  .btn {
    padding: 8px 20px;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .btn:hover {
    opacity: 0.85;
  }

  .btn.cancel {
    background: #333;
    color: #aaa;
  }

  .btn.start {
    background: white;
    color: black;
  }
</style>
