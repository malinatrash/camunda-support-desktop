<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  interface Props {
    items: T[];
    itemHeight: number;
    overscan?: number;
    row: Snippet<[T, number]>;
    class?: string;
    key?: (item: T) => string;
  }

  let { items, itemHeight, overscan = 6, row, class: className = "", key }: Props = $props();

  let scrollTop = $state(0);
  let viewportHeight = $state(0);

  const start = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - overscan));
  const end = $derived(Math.min(items.length, Math.ceil((scrollTop + viewportHeight) / itemHeight) + overscan));
  const visible = $derived(items.slice(start, end).map((item, offset) => ({ item, index: start + offset })));

  function onScroll(event: Event) {
    scrollTop = (event.currentTarget as HTMLElement).scrollTop;
  }
</script>

<div class="viewport scroll {className}" onscroll={onScroll} bind:clientHeight={viewportHeight}>
  <div class="spacer" style:height="{items.length * itemHeight}px">
    {#each visible as entry (key ? key(entry.item) : entry.index)}
      <div class="slot" style:transform="translateY({entry.index * itemHeight}px)" style:height="{itemHeight}px">
        {@render row(entry.item, entry.index)}
      </div>
    {/each}
  </div>
</div>

<style>
  .viewport {
    position: relative;
    height: 100%;
    min-height: 0;
    overflow: auto;
    contain: strict;
  }
  .spacer {
    position: relative;
    width: 100%;
  }
  .slot {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    will-change: transform;
  }
</style>
