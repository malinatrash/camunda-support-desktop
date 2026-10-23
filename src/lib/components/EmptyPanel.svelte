<script lang="ts">
  import Button from "./Button.svelte";
  import { rise } from "../motion";

  interface Props {
    title: string;
    description: string;
    actionLabel?: string;
    onaction?: () => void;
    tone?: "neutral" | "danger";
    compact?: boolean;
  }

  let { title, description, actionLabel, onaction, tone = "neutral", compact = false }: Props = $props();
</script>

<div class="empty card" class:danger={tone === "danger"} class:compact in:rise>
  <div class="glyph" aria-hidden="true"></div>
  <h3 class="title">{title}</h3>
  <p class="description muted selectable">{description}</p>
  {#if actionLabel && onaction}
    <Button variant={tone === "danger" ? "secondary" : "primary"} onclick={onaction}>{actionLabel}</Button>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 26px 22px;
    text-align: center;
  }
  .compact {
    padding: 16px;
  }
  .glyph {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: linear-gradient(135deg, var(--accent-soft), var(--surface-3));
    border: 1px solid var(--border);
    margin-bottom: 2px;
  }
  .danger .glyph {
    background: linear-gradient(135deg, var(--danger-soft), var(--surface-3));
  }
  .title {
    font-size: 13.5px;
    font-weight: 650;
  }
  .description {
    font-size: 12px;
    max-width: 640px;
    white-space: pre-wrap;
    line-height: 1.5;
  }
</style>
