<script lang="ts">
  import type { Snippet } from "svelte";

  let { checked = $bindable(false), disabled = false, tone = "neutral", children }: {
    checked?: boolean;
    disabled?: boolean;
    tone?: "neutral" | "danger";
    children?: Snippet;
  } = $props();
</script>

<label class="check tone-{tone}" class:disabled>
  <input type="checkbox" bind:checked {disabled} />
  <span class="box" aria-hidden="true">
    <svg viewBox="0 0 12 12" width="12" height="12"><path d="M2.5 6.5l2.3 2.3L9.5 4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
  </span>
  <span class="text">{@render children?.()}</span>
</label>

<style>
  .check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 12px;
  }
  .check.disabled {
    opacity: 0.6;
    cursor: default;
  }
  input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .box {
    width: 16px;
    height: 16px;
    border-radius: 5px;
    border: 1.5px solid var(--border-strong);
    background: var(--surface);
    display: inline-grid;
    place-items: center;
    color: transparent;
    transition:
      background-color var(--dur-fast) var(--ease-out),
      border-color var(--dur-fast) var(--ease-out),
      transform var(--dur-fast) var(--ease-spring);
  }
  .box svg {
    transform: scale(0.4);
    transition: transform var(--dur-base) var(--ease-spring);
  }
  input:checked + .box {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .tone-danger input:checked + .box {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
  input:checked + .box svg {
    transform: scale(1);
  }
  input:focus-visible + .box {
    box-shadow: var(--focus-ring);
  }
  .tone-danger .text {
    color: var(--danger);
    font-weight: 700;
  }
</style>
