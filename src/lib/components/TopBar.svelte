<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../state/app.svelte";
  import { palette } from "../state/palette.svelte";
  import { updates } from "../state/updates.svelte";
  import { api } from "../api";
  import { cockpitDashboardUrl } from "../connection";
  import { clickOutside } from "../actions";
  import { environmentTone, ENVIRONMENT_LABEL } from "../format";
  import { pop } from "../motion";
  import { DESTINATIONS, type AppDestination } from "../types";

  const connection = $derived(app.selectedConnection);
  const cockpitUrl = $derived(connection ? cockpitDashboardUrl(connection) : null);
  const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  const modifier = isMac ? "⌘" : "Ctrl";
  const sections = DESTINATIONS.filter((item) => item.id !== "Settings");
  const current = $derived<AppDestination>(
    app.destination === "ProcessDefinition" || app.destination === "ProcessInstance" ? "Overview" : app.destination,
  );
  let switcherOpen = $state(false);
</script>

<header class="topbar" data-tauri-drag-region>
  <div class="switcher" use:clickOutside={() => (switcherOpen = false)}>
    <button type="button" class="connection" onclick={() => (switcherOpen = !switcherOpen)}>
      <span class="env-dot tone-{connection ? environmentTone(connection.environment) : 'neutral'}"></span>
      <span class="name truncate">{connection?.name ?? "Нет подключения"}</span>
      <Icon name="chevron" size={13} class="chevron {switcherOpen ? 'flipped' : ''}" />
    </button>
    {#if switcherOpen}
      <div class="menu" transition:pop={{ duration: 140, start: 0.98 }}>
        {#each app.connections as item (item.id)}
          <button type="button" class="menu-item" class:active={item.id === app.selectedConnectionId} onclick={() => { app.selectConnection(item.id); switcherOpen = false; }}>
            <span class="env-dot tone-{environmentTone(item.environment)}"></span>
            <span class="col grow">
              <span class="truncate">{item.name}</span>
              <span class="faint mono truncate small">{ENVIRONMENT_LABEL[item.environment].label}{item.engineVersion ? ` · ${item.engineVersion}` : ""}</span>
            </span>
            {#if item.id === app.selectedConnectionId}<Icon name="check" size={14} />{/if}
          </button>
        {/each}
        <div class="divider"></div>
        {#if cockpitUrl}
          <button type="button" class="menu-item" onclick={() => { api.openInBrowser(cockpitUrl); switcherOpen = false; }}>
            <Icon name="external" size={14} /><span>Открыть Cockpit</span>
          </button>
        {/if}
        <button type="button" class="menu-item" onclick={() => { app.openConnectionDialog(); switcherOpen = false; }}>
          <Icon name="plus" size={14} /><span>Добавить подключение</span>
        </button>
      </div>
    {/if}
  </div>

  <nav class="nav">
    {#each sections as item (item.id)}
      <button type="button" class="nav-item" class:active={current === item.id} title="{item.description} · {modifier} {item.shortcut}" onclick={() => app.navigate(item.id)}>
        {item.title}
      </button>
    {/each}
  </nav>

  <span class="grow" data-tauri-drag-region></span>

  <button type="button" class="search" onclick={() => palette.show()}>
    <Icon name="search" size={14} />
    <span>Найти заявку</span>
    <span class="hint">{modifier}K</span>
  </button>

  <button type="button" class="icon-button" class:active={current === "Settings"} title="Настройки · {modifier} ," onclick={() => app.navigate("Settings")}>
    <Icon name="settings" size={16} />
    {#if updates.visible}<span class="badge" aria-label="Доступно обновление"></span>{/if}
  </button>
</header>

<style>
  .topbar {
    height: var(--topbar-height);
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .switcher {
    position: relative;
  }
  .connection {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 240px;
    height: 30px;
    padding: 0 8px;
    margin-left: -8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    font-weight: 600;
  }
  .connection:hover {
    background: var(--surface-2);
  }
  .env-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
    background: var(--neutral);
  }
  .env-dot.tone-danger { background: var(--danger); }
  .env-dot.tone-warning { background: var(--warning); }
  .env-dot.tone-development { background: var(--info); }
  :global(.chevron) {
    color: var(--ink-3);
    transition: transform var(--dur-base) var(--ease-out);
  }
  :global(.chevron.flipped) {
    transform: rotate(180deg);
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: -8px;
    z-index: 40;
    width: 300px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-3);
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 8px;
    border: 0;
    border-radius: var(--radius-xs);
    background: transparent;
    text-align: left;
  }
  .menu-item:hover {
    background: var(--surface-2);
  }
  .menu-item.active {
    color: var(--accent);
  }
  .menu .divider {
    margin: 4px 0;
  }
  .small {
    font-size: var(--text-xs);
  }
  .nav {
    display: flex;
    gap: 2px;
  }
  .nav-item {
    height: 30px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
    font-weight: 500;
  }
  .nav-item:hover {
    color: var(--ink);
  }
  .nav-item:hover {
    background: var(--row-hover);
  }
  .nav-item.active {
    color: var(--ink);
    font-weight: 600;
    background: var(--surface-3);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 240px;
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--ink-3);
  }
  .search:hover {
    border-color: var(--border-strong);
  }
  .search span:nth-child(2) {
    flex: 1;
    text-align: left;
  }
  .hint {
    font-size: var(--text-xs);
  }
  .icon-button {
    position: relative;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
  }
  .icon-button:hover,
  .icon-button.active {
    color: var(--ink);
    background: var(--surface-3);
  }
  .badge {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
</style>
