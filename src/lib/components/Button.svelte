<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  interface Props extends HTMLButtonAttributes {
    variant?: "primary" | "secondary" | "ghost" | "danger" | "subtle";
    size?: "sm" | "md";
    loading?: boolean;
    block?: boolean;
    children?: Snippet;
  }

  let {
    variant = "secondary",
    size = "md",
    loading = false,
    block = false,
    disabled = false,
    type = "button",
    children,
    class: className = "",
    ...rest
  }: Props = $props();
</script>

<button class="btn btn-{variant} btn-{size} {className}" class:block class:loading disabled={disabled || loading} {type} {...rest}>
  {#if loading}<span class="btn-spinner" aria-hidden="true"></span>{/if}
  <span class="btn-label">{@render children?.()}</span>
</button>

<style>
  .btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    font-weight: 600;
    white-space: nowrap;
    line-height: 1;
    transition:
      background-color var(--dur-fast) var(--ease-out),
      border-color var(--dur-fast) var(--ease-out),
      color var(--dur-fast) var(--ease-out),
      box-shadow var(--dur-fast) var(--ease-out),
      transform var(--dur-fast) var(--ease-out);
    will-change: transform;
  }
  .btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .btn:active:not(:disabled) {
    transform: scale(0.97);
  }
  .btn:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .btn-md {
    height: 32px;
    padding: 0 13px;
    font-size: 12.5px;
  }
  .btn-sm {
    height: 26px;
    padding: 0 10px;
    font-size: 11.5px;
    border-radius: var(--radius-xs);
  }
  .block {
    width: 100%;
  }
  .btn-primary {
    background: var(--accent);
    color: var(--on-accent);
    box-shadow: var(--shadow-1);
  }
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-strong);
    box-shadow: var(--shadow-2);
  }
  :global([data-theme="dark"]) .btn-primary {
    color: var(--on-accent);
  }
  .btn-secondary {
    background: var(--surface);
    border-color: var(--border-strong);
    color: var(--ink);
    box-shadow: var(--shadow-1);
  }
  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-2);
    border-color: var(--ink-3);
  }
  .btn-ghost {
    background: transparent;
    color: var(--ink-2);
  }
  .btn-ghost:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--ink);
  }
  .btn-subtle {
    background: var(--accent-soft);
    color: var(--accent-ink);
  }
  .btn-subtle:hover:not(:disabled) {
    background: var(--accent-soft-2);
  }
  .btn-danger {
    background: var(--danger);
    color: #fff;
  }
  .btn-danger:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .btn-spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    animation: spin 0.7s linear infinite;
  }
  .btn-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
