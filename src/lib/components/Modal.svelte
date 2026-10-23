<script lang="ts">
  import type { Snippet } from "svelte";
  import { portal } from "../actions";
  import { fadeFast, pop } from "../motion";

  interface Props {
    open: boolean;
    title: string;
    subtitle?: string;
    width?: string;
    height?: string;
    onclose: () => void;
    closable?: boolean;
    children: Snippet;
    footer?: Snippet;
  }

  let { open, title, subtitle, width = "600px", height, onclose, closable = true, children, footer }: Props = $props();

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape" && closable) onclose();
  }
</script>

<svelte:window onkeydown={open ? onKey : undefined} />

{#if open}
  <div class="backdrop" use:portal transition:fadeFast={{ duration: 160 }} onpointerdown={() => closable && onclose()} role="presentation">
    <div
      class="dialog card"
      style:width
      style:height
      transition:pop={{ duration: 200, start: 0.96 }}
      onpointerdown={(event) => event.stopPropagation()}
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-label={title}
    >
      <header class="head">
        <div class="col grow">
          <h2 class="title">{title}</h2>
          {#if subtitle}<p class="subtitle muted">{subtitle}</p>{/if}
        </div>
        {#if closable}
          <button type="button" class="close" onclick={onclose} aria-label="Закрыть">×</button>
        {/if}
      </header>
      <div class="body scroll">
        {@render children()}
      </div>
      {#if footer}
        <footer class="foot">{@render footer()}</footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(6, 10, 8, 0.42);
    backdrop-filter: blur(6px) saturate(120%);
    -webkit-backdrop-filter: blur(6px) saturate(120%);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    max-width: calc(100vw - 48px);
    max-height: calc(100vh - 48px);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-3);
    transform-origin: center;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 18px 20px 12px;
  }
  .title {
    font-size: 18px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .subtitle {
    margin-top: 3px;
    font-size: 12px;
  }
  .close {
    border: 0;
    background: transparent;
    color: var(--ink-3);
    font-size: 22px;
    line-height: 1;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-xs);
    transition: background-color var(--dur-fast), color var(--dur-fast);
  }
  .close:hover {
    background: var(--surface-3);
    color: var(--ink);
  }
  .body {
    padding: 4px 20px 16px;
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px 16px;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
</style>
