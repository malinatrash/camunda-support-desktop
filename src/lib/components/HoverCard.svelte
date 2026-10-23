<script lang="ts">
  import type { Snippet } from "svelte";
  import { portal } from "../actions";
  import { pop } from "../motion";

  interface Props {
    children: Snippet;
    card: Snippet;
    class?: string;
    width?: number;
  }

  let { children, card, class: className = "", width = 330 }: Props = $props();

  let visible = $state(false);
  let x = $state(0);
  let y = $state(0);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function place(event: PointerEvent) {
    const margin = 12;
    const maxX = window.innerWidth - width - margin;
    const maxY = window.innerHeight - 320;
    x = Math.min(event.clientX + 16, Math.max(margin, maxX));
    y = Math.min(event.clientY + 16, Math.max(margin, maxY));
  }

  function enter(event: PointerEvent) {
    place(event);
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => (visible = true), 180);
  }

  function leave() {
    if (timer) clearTimeout(timer);
    timer = null;
    visible = false;
  }
</script>

<div class={className} onpointerenter={enter} onpointermove={place} onpointerleave={leave} onpointerdown={leave} role="presentation">
  {@render children()}
</div>

{#if visible}
  <div class="hover-card card" use:portal style:left="{x}px" style:top="{y}px" style:width="{width}px" transition:pop={{ duration: 150, start: 0.97 }}>
    {@render card()}
  </div>
{/if}

<style>
  .hover-card {
    position: fixed;
    z-index: 90;
    pointer-events: none;
    box-shadow: var(--shadow-3);
    border-color: var(--border-strong);
    transform-origin: top left;
  }
</style>
