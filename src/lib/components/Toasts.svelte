<script lang="ts">
  import { flip } from "svelte/animate";
  import { fly } from "svelte/transition";
  import { toasts } from "../state/toasts.svelte";
  import { easeOutQuint, fadeFast } from "../motion";
</script>

<div class="toasts" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <button
      type="button"
      class="toast tone-{toast.tone} selectable"
      animate:flip={{ duration: 240, easing: easeOutQuint }}
      in:fly={{ y: 16, duration: 260, easing: easeOutQuint }}
      out:fadeFast
      onclick={() => toasts.dismiss(toast.id)}
    >
      <span class="dot"></span>
      <span>{toast.message}</span>
    </button>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    z-index: 200;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 560px;
    padding: 9px 14px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-3);
    font-size: 12px;
    color: var(--ink);
    text-align: left;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    flex: none;
  }
  .tone-success .dot {
    background: var(--healthy);
  }
  .tone-danger .dot {
    background: var(--danger);
  }
</style>
