<script lang="ts">
  import Icon from "./Icon.svelte";
  import StateBadge from "./StateBadge.svelte";
  import { portal } from "../actions";
  import { app } from "../state/app.svelte";
  import { palette } from "../state/palette.svelte";
  import { recents } from "../state/recents.svelte";
  import { theme } from "../state/theme.svelte";
  import { api, errorMessage } from "../api";
  import { fadeFast, pop } from "../motion";
  import { contains } from "../format";
  import { SIDEBAR_DESTINATIONS, type ProcessInstanceSummary } from "../types";

  let { onOpenLink }: { onOpenLink: (raw: string) => void } = $props();

  type Item =
    | { kind: "instance"; id: string; instance: ProcessInstanceSummary }
    | { kind: "recent"; id: string; connectionId: string; processInstanceId: string; title: string; subtitle: string; pinned: boolean }
    | { kind: "link"; id: string; raw: string }
    | { kind: "command"; id: string; title: string; hint: string; icon: string; run: () => void };

  let query = $state("");
  let results = $state<ProcessInstanceSummary[]>([]);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let highlighted = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let listElement = $state<HTMLDivElement | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;
  let serial = 0;

  const isLink = $derived(/^(https?:\/\/|camunda-support:)/i.test(query.trim()));

  $effect(() => {
    if (palette.open) {
      query = palette.initialQuery;
      results = [];
      searchError = null;
      highlighted = 0;
      requestAnimationFrame(() => input?.focus());
    }
  });

  $effect(() => {
    const needle = query.trim();
    const connection = app.selectedConnection;
    if (timer) clearTimeout(timer);
    if (!palette.open || !connection || needle.length < 2 || isLink) {
      results = [];
      searching = false;
      return;
    }
    const current = ++serial;
    searching = true;
    timer = setTimeout(async () => {
      try {
        const found = await api.searchInstancesByBusinessKey(connection, needle);
        if (current === serial) {
          results = found;
          searchError = null;
        }
      } catch (error) {
        if (current === serial) searchError = errorMessage(error);
      } finally {
        if (current === serial) searching = false;
      }
    }, 260);
  });

  const commands = $derived.by<Item[]>(() => {
    const list: Item[] = SIDEBAR_DESTINATIONS.map((item) => ({
      kind: "command",
      id: `nav:${item.id}`,
      title: item.title,
      hint: item.description,
      icon: item.icon,
      run: () => app.navigate(item.id),
    }));
    for (const connection of app.connections) {
      list.push({
        kind: "command",
        id: `conn:${connection.id}`,
        title: `Подключение: ${connection.name}`,
        hint: connection.restUrl,
        icon: "plug",
        run: () => app.selectConnection(connection.id),
      });
    }
    list.push({ kind: "command", id: "theme", title: theme.mode === "Dark" ? "Светлая тема" : "Тёмная тема", hint: "Переключить оформление", icon: theme.mode === "Dark" ? "sun" : "moon", run: () => theme.set(theme.mode === "Dark" ? "Light" : "Dark") });
    list.push({ kind: "command", id: "refresh", title: "Обновить данные", hint: "Перезагрузить текущий экран", icon: "refresh", run: () => palette.requestRefresh() });
    list.push({ kind: "command", id: "add-connection", title: "Добавить подключение", hint: "Новое окружение Camunda", icon: "plus", run: () => app.openConnectionDialog() });
    return list;
  });

  const items = $derived.by<Item[]>(() => {
    const needle = query.trim();
    const list: Item[] = [];
    if (isLink) list.push({ kind: "link", id: "link", raw: needle });
    for (const instance of results) list.push({ kind: "instance", id: `inst:${instance.id}`, instance });
    const recentItems = [...recents.pinned, ...recents.recent]
      .filter((item) => !needle || contains(item.businessKey, needle) || contains(item.processInstanceId, needle) || contains(item.processDefinitionKey, needle))
      .filter((item) => !results.some((instance) => instance.id === item.processInstanceId))
      .slice(0, needle ? 6 : 8)
      .map<Item>((item) => ({
        kind: "recent",
        id: `recent:${item.connectionId}:${item.processInstanceId}`,
        connectionId: item.connectionId,
        processInstanceId: item.processInstanceId,
        title: item.businessKey ?? item.processInstanceId,
        subtitle: [item.processDefinitionKey, item.connectionName].filter(Boolean).join(" · "),
        pinned: item.pinned,
      }));
    list.push(...recentItems);
    const matchingCommands = needle ? commands.filter((item) => item.kind === "command" && (contains(item.title, needle) || contains(item.hint, needle))) : commands;
    list.push(...matchingCommands.slice(0, needle ? 5 : 20));
    return list;
  });

  $effect(() => {
    items;
    highlighted = 0;
  });

  function run(item: Item) {
    palette.hide();
    switch (item.kind) {
      case "instance":
        app.openProcessInstance(item.instance.id);
        break;
      case "recent":
        app.openProcessInstance(item.processInstanceId, item.connectionId);
        break;
      case "link":
        onOpenLink(item.raw);
        break;
      case "command":
        item.run();
        break;
    }
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      palette.hide();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      highlighted = Math.min(items.length - 1, highlighted + 1);
      scrollToHighlighted();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      highlighted = Math.max(0, highlighted - 1);
      scrollToHighlighted();
    } else if (event.key === "Enter") {
      event.preventDefault();
      const item = items[highlighted];
      if (item) run(item);
    }
  }

  function scrollToHighlighted() {
    (listElement?.querySelectorAll(".item")[highlighted] as HTMLElement | undefined)?.scrollIntoView({ block: "nearest" });
  }

  function sectionLabel(index: number): string | null {
    const item = items[index];
    const previous = items[index - 1];
    if (previous && previous.kind === item.kind) return null;
    switch (item.kind) {
      case "link":
        return "Ссылка";
      case "instance":
        return "Заявки";
      case "recent":
        return "Недавние и закреплённые";
      default:
        return "Команды";
    }
  }
</script>

{#if palette.open}
  <div class="backdrop" use:portal transition:fadeFast={{ duration: 140 }} onpointerdown={() => palette.hide()} role="presentation">
    <div class="palette card" transition:pop={{ duration: 180, start: 0.97 }} onpointerdown={(event) => event.stopPropagation()} role="dialog" aria-label="Командная строка" tabindex="-1" onkeydown={onKey}>
      <div class="input-row">
        <Icon name="search" size={17} class="search-icon" />
        <input
          class="input selectable"
          placeholder={app.selectedConnection ? "Бизнес-ключ, ID заявки, ссылка или команда…" : "Команда или ссылка… (нет подключения)"}
          bind:value={query}
          bind:this={input}
          spellcheck="false"
          autocomplete="off"
        />
        {#if searching}<span class="spinner" aria-hidden="true"></span>{/if}
        <span class="kbd">Esc</span>
      </div>
      <div class="list scroll" bind:this={listElement}>
        {#if searchError}
          <div class="error selectable">{searchError}</div>
        {/if}
        {#each items as item, index (item.id)}
          {@const label = sectionLabel(index)}
          {#if label}<div class="section">{label}</div>{/if}
          <button type="button" class="item" class:highlighted={index === highlighted} onmouseenter={() => (highlighted = index)} onclick={() => run(item)}>
            {#if item.kind === "instance"}
              <span class="item-icon"><Icon name="file" size={15} /></span>
              <span class="col grow">
                <span class="row">
                  <span class="title truncate" class:mono={!item.instance.businessKey}>{item.instance.businessKey ?? item.instance.id}</span>
                  <StateBadge suspended={item.instance.suspended} ended={item.instance.ended} labels="instance" />
                </span>
                <span class="subtitle mono truncate">{item.instance.definitionName ?? item.instance.definitionKey} · {item.instance.id}{item.instance.startTime ? ` · ${item.instance.startTime.slice(0, 16).replace("T", " ")}` : ""}</span>
              </span>
            {:else if item.kind === "recent"}
              <span class="item-icon"><Icon name={item.pinned ? "star" : "history"} size={15} /></span>
              <span class="col grow">
                <span class="title truncate">{item.title}</span>
                <span class="subtitle truncate">{item.subtitle}</span>
              </span>
            {:else if item.kind === "link"}
              <span class="item-icon"><Icon name="link" size={15} /></span>
              <span class="col grow">
                <span class="title">Открыть заявку по ссылке</span>
                <span class="subtitle mono truncate">{item.raw}</span>
              </span>
            {:else}
              <span class="item-icon"><Icon name={item.icon} size={15} /></span>
              <span class="col grow">
                <span class="title">{item.title}</span>
                <span class="subtitle truncate">{item.hint}</span>
              </span>
            {/if}
            <span class="enter kbd">↵</span>
          </button>
        {:else}
          <div class="empty muted">
            {#if searching}
              Ищем заявки…
            {:else if query.trim().length >= 2}
              Ничего не найдено. Поиск по переменным — на экране «Поиск».
            {:else}
              Введите бизнес-ключ, ID или ссылку
            {/if}
          </div>
        {/each}
      </div>
      <div class="foot">
        <span><span class="kbd">↑</span><span class="kbd">↓</span> навигация</span>
        <span><span class="kbd">↵</span> открыть</span>
        <span class="grow"></span>
        <span class="faint">Ищет по бизнес-ключу среди запущенных и завершённых заявок</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 120;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: rgba(8, 10, 16, 0.4);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }
  .palette {
    width: 680px;
    max-width: calc(100vw - 40px);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow-3);
    border-color: var(--border-strong);
    transform-origin: top center;
  }
  .input-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  :global(.search-icon) {
    color: var(--ink-3);
  }
  .input {
    flex: 1;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 15px;
    height: 28px;
  }
  .input::placeholder {
    color: var(--ink-3);
  }
  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--accent) 25%, transparent);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  .list {
    flex: 1;
    min-height: 0;
    padding: 6px;
  }
  .section {
    padding: 8px 10px 4px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    transition: background-color var(--dur-fast);
  }
  .item.highlighted {
    background: var(--accent-soft);
  }
  .item-icon {
    width: 28px;
    height: 28px;
    border-radius: 7px;
    display: grid;
    place-items: center;
    background: var(--surface-3);
    color: var(--ink-2);
    flex: none;
  }
  .highlighted .item-icon {
    background: var(--accent-soft-2);
    color: var(--accent-ink);
  }
  .title {
    font-size: 13px;
    font-weight: 500;
  }
  .subtitle {
    font-size: 10.5px;
    color: var(--ink-3);
  }
  .enter {
    opacity: 0;
    transition: opacity var(--dur-fast);
  }
  .highlighted .enter {
    opacity: 1;
  }
  .empty {
    padding: 28px;
    text-align: center;
    font-size: 12.5px;
  }
  .error {
    margin: 4px;
    padding: 8px 10px;
    border-radius: var(--radius-xs);
    background: var(--danger-soft);
    color: var(--danger);
    font-size: 11.5px;
    white-space: pre-wrap;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
    font-size: 10.5px;
    color: var(--ink-2);
  }
  .foot .kbd {
    margin-right: 3px;
  }
</style>
