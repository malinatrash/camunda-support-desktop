<script lang="ts">
  import AnimatedNumber from "./AnimatedNumber.svelte";

  let { label, value, caption, tone = "primary", numeric = true }: {
    label: string;
    value: string | number;
    caption: string;
    tone?: "primary" | "healthy" | "danger" | "warning" | "neutral";
    numeric?: boolean;
  } = $props();
</script>

<div class="metric card">
  <span class="eyebrow">{label}</span>
  <span class="value mono">
    {#if numeric && typeof value === "number"}
      <AnimatedNumber {value} />
    {:else}
      {value}
    {/if}
  </span>
  <span class="caption tone-{tone}">{caption}</span>
</div>

<style>
  .metric {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 11px 13px;
    min-width: 0;
    transition: transform var(--dur-base) var(--ease-out), box-shadow var(--dur-base) var(--ease-out);
  }
  .metric:hover {
    box-shadow: var(--shadow-2);
  }
  .value {
    font-size: 19px;
    font-weight: 620;
    letter-spacing: -0.01em;
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .caption {
    font-size: 10.5px;
    font-weight: 500;
  }
  .tone-primary { color: var(--accent); }
  .tone-healthy { color: var(--healthy); }
  .tone-danger { color: var(--danger); }
  .tone-warning { color: var(--warning); }
  .tone-neutral { color: var(--ink-2); }
</style>
