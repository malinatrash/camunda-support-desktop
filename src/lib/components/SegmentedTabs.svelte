<script lang="ts" generics="T extends string">
  interface Item {
    id: T;
    label: string;
    badge?: string | number;
  }

  interface Props {
    items: Item[];
    selected: T;
    onselect: (id: T) => void;
    size?: "sm" | "md";
    tone?: "neutral" | "primary";
  }

  let { items, selected, onselect, size = "md", tone = "neutral" }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let buttons = $state<Record<string, HTMLButtonElement>>({});
  let indicator = $state({ left: 0, width: 0, ready: false });

  function measure() {
    const button = buttons[selected];
    if (!button || !host) return;
    indicator = { left: button.offsetLeft, width: button.offsetWidth, ready: true };
  }

  $effect(() => {
    selected;
    items;
    measure();
  });

  $effect(() => {
    if (!host) return;
    const observer = new ResizeObserver(() => measure());
    observer.observe(host);
    return () => observer.disconnect();
  });
</script>

<div class="segmented segmented-{size} tone-{tone}" bind:this={host} role="tablist">
  <span
    class="indicator"
    class:ready={indicator.ready}
    style:transform="translateX({indicator.left}px)"
    style:width="{indicator.width}px"
    aria-hidden="true"
  ></span>
  {#each items as item (item.id)}
    <button
      type="button"
      role="tab"
      class="tab"
      class:active={item.id === selected}
      aria-selected={item.id === selected}
      bind:this={buttons[item.id]}
      onclick={() => onselect(item.id)}
    >
      {item.label}
      {#if item.badge !== undefined}<span class="badge">{item.badge}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .segmented {
    position: relative;
    display: inline-flex;
    gap: 18px;
    isolation: isolate;
  }
  .indicator {
    position: absolute;
    bottom: 0;
    left: 0;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
    opacity: 0;
  }
  .indicator.ready {
    opacity: 1;
    transition:
      transform var(--dur-base) var(--ease-out),
      width var(--dur-base) var(--ease-out);
  }
  .tab {
    position: relative;
    border: 0;
    background: transparent;
    padding: 0;
    color: var(--ink-2);
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: color var(--dur-fast) var(--ease-out);
  }
  .segmented-md .tab {
    height: 32px;
    font-size: var(--text-base);
  }
  .segmented-sm .tab {
    height: 28px;
    font-size: var(--text-md);
  }
  .tab:hover,
  .tab.active {
    color: var(--ink);
  }
  .badge {
    color: var(--ink-3);
    font-weight: 400;
  }
</style>
