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
    gap: 2px;
    padding: 3px;
    border-radius: 10px;
    background: var(--surface-3);
    isolation: isolate;
  }
  .indicator {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 0;
    border-radius: 8px;
    background: var(--surface);
    box-shadow: var(--shadow-1);
    opacity: 0;
    z-index: 0;
  }
  .indicator.ready {
    opacity: 1;
    transition:
      transform var(--dur-slow) var(--ease-spring),
      width var(--dur-slow) var(--ease-out);
  }
  .tab {
    position: relative;
    z-index: 1;
    border: 0;
    background: transparent;
    border-radius: 8px;
    color: var(--ink-2);
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: color var(--dur-fast) var(--ease-out);
  }
  .segmented-md .tab {
    height: 30px;
    padding: 0 14px;
    font-size: 12px;
  }
  .segmented-sm .tab {
    height: 24px;
    padding: 0 10px;
    font-size: 11px;
  }
  .tab:hover {
    color: var(--ink);
  }
  .tab.active {
    color: var(--ink);
    font-weight: 600;
  }
  .tone-primary .tab.active {
    color: var(--accent-ink);
  }
  .badge {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--ink-3);
  }
  .tab.active .badge {
    color: var(--accent);
  }
</style>
