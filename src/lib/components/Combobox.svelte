<script lang="ts">
  import { clickOutside, focusOnMount } from "../actions";
  import { pop } from "../motion";
  import { contains } from "../format";

  export interface ComboboxItem {
    id: string;
    title: string;
    subtitle?: string;
    mono?: boolean;
  }

  interface Props {
    items: ComboboxItem[];
    selectedId: string | null;
    onselect: (item: ComboboxItem) => void;
    label: string;
    placeholder?: string;
    filterPlaceholder?: string;
    disabled?: boolean;
    loading?: boolean;
    loadingText?: string;
    errorText?: string | null;
    emptyText?: string;
    width?: string;
    allowClear?: boolean;
    clearLabel?: string;
    onclear?: () => void;
  }

  let {
    items,
    selectedId,
    onselect,
    label,
    placeholder = "Выберите",
    filterPlaceholder = "Фильтр",
    disabled = false,
    loading = false,
    loadingText = "Загружаем…",
    errorText = null,
    emptyText = "Ничего не найдено",
    width = "440px",
    allowClear = false,
    clearLabel = "Сбросить",
    onclear,
  }: Props = $props();

  let open = $state(false);
  let filter = $state("");
  let highlighted = $state(0);
  let listElement = $state<HTMLDivElement | null>(null);

  const selected = $derived(items.find((item) => item.id === selectedId) ?? null);
  const filtered = $derived(
    filter.trim() === "" ? items : items.filter((item) => contains(item.title, filter) || contains(item.subtitle, filter)),
  );
  const enabled = $derived(!disabled && !loading && errorText == null && items.length > 0);

  function toggle() {
    if (!enabled) return;
    open = !open;
    if (open) {
      filter = "";
      highlighted = Math.max(0, filtered.findIndex((item) => item.id === selectedId));
    }
  }

  function choose(item: ComboboxItem) {
    onselect(item);
    open = false;
  }

  function onKey(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === "Escape") {
      open = false;
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      highlighted = Math.min(filtered.length - 1, highlighted + 1);
      scrollToHighlighted();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      highlighted = Math.max(0, highlighted - 1);
      scrollToHighlighted();
    } else if (event.key === "Enter") {
      event.preventDefault();
      const item = filtered[highlighted];
      if (item) choose(item);
    }
  }

  function scrollToHighlighted() {
    const node = listElement?.children[highlighted] as HTMLElement | undefined;
    node?.scrollIntoView({ block: "nearest" });
  }

  $effect(() => {
    filter;
    highlighted = 0;
  });
</script>

<div class="combobox" use:clickOutside={() => (open = false)} onkeydown={onKey} role="presentation">
  <button type="button" class="trigger" class:open disabled={!enabled} onclick={toggle}>
    <span class="col grow">
      <span class="eyebrow">{label}</span>
      <span class="value truncate" class:placeholder={!selected} class:mono={selected?.mono}>
        {#if loading}
          {loadingText}
        {:else if errorText}
          Не удалось загрузить
        {:else if selected}
          {selected.title}
        {:else}
          {placeholder}
        {/if}
      </span>
      {#if selected?.subtitle}
        <span class="subtitle truncate mono">{selected.subtitle}</span>
      {/if}
    </span>
    <span class="chevron" class:flipped={open} aria-hidden="true">⌄</span>
  </button>

  {#if open}
    <div class="panel card" style:width transition:pop={{ duration: 160, start: 0.97 }}>
      <div class="panel-filter">
        <input
          class="filter-input selectable"
          placeholder={filterPlaceholder}
          bind:value={filter}
          spellcheck="false"
          use:focusOnMount={true}
        />
        <span class="faint count">Найдено: {filtered.length}</span>
      </div>
      <div class="list scroll" bind:this={listElement}>
        {#if allowClear}
          <button type="button" class="item" onclick={() => { onclear?.(); open = false; }}>
            <span class="item-title">{clearLabel}</span>
          </button>
        {/if}
        {#each filtered as item, index (item.id)}
          <button
            type="button"
            class="item"
            class:active={item.id === selectedId}
            class:highlighted={index === highlighted}
            onmouseenter={() => (highlighted = index)}
            onclick={() => choose(item)}
          >
            <span class="item-title truncate" class:mono={item.mono}>{item.title}</span>
            {#if item.subtitle}<span class="item-subtitle truncate mono">{item.subtitle}</span>{/if}
          </button>
        {:else}
          <div class="empty muted">{emptyText}</div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .combobox {
    position: relative;
    min-width: 0;
  }
  .trigger {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    height: 52px;
    padding: 0 12px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    transition:
      border-color var(--dur-fast) var(--ease-out),
      box-shadow var(--dur-fast) var(--ease-out),
      background-color var(--dur-fast) var(--ease-out);
  }
  .trigger:hover:not(:disabled) {
    border-color: var(--ink-3);
  }
  .trigger.open,
  .trigger:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  .trigger:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .value {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink);
    line-height: 1.3;
  }
  .value.placeholder {
    color: var(--ink-3);
    font-weight: 500;
  }
  .subtitle {
    font-size: 10px;
    color: var(--ink-2);
  }
  .chevron {
    color: var(--ink-3);
    font-size: 16px;
    line-height: 1;
    transition: transform var(--dur-base) var(--ease-spring);
  }
  .chevron.flipped {
    transform: translateY(2px) rotate(180deg);
  }
  .panel {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 40;
    max-width: calc(100vw - 48px);
    box-shadow: var(--shadow-3);
    transform-origin: top left;
    overflow: hidden;
  }
  .panel-filter {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }
  .filter-input {
    flex: 1;
    height: 30px;
    padding: 0 10px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    outline: none;
  }
  .filter-input:focus {
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  .count {
    font-size: 10.5px;
    white-space: nowrap;
    padding-right: 4px;
  }
  .list {
    max-height: 420px;
    padding: 6px;
  }
  .item {
    display: flex;
    flex-direction: column;
    width: 100%;
    gap: 2px;
    padding: 7px 9px;
    border: 0;
    border-radius: var(--radius-xs);
    background: transparent;
    text-align: left;
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  .item.highlighted {
    background: var(--surface-3);
  }
  .item.active {
    background: var(--accent-soft);
  }
  .item.active .item-title {
    color: var(--accent-ink);
  }
  .item-title {
    font-size: 12.5px;
    font-weight: 500;
  }
  .item-subtitle {
    font-size: 10px;
    color: var(--ink-2);
  }
  .empty {
    padding: 14px;
    text-align: center;
    font-size: 12px;
  }
</style>
