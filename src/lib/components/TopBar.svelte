<script lang="ts">
  import Icon from "./Icon.svelte";
  import EnvironmentBadge from "./EnvironmentBadge.svelte";
  import { app } from "../state/app.svelte";
  import { palette } from "../state/palette.svelte";
  import { api } from "../api";
  import { cockpitDashboardUrl } from "../connection";
  import { clickOutside } from "../actions";
  import { environmentTone, ENVIRONMENT_LABEL } from "../format";
  import { pop } from "../motion";

  const connection = $derived(app.selectedConnection);
  const cockpitUrl = $derived(connection ? cockpitDashboardUrl(connection) : null);
  const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  let switcherOpen = $state(false);
</script>

<header class="topbar" data-tauri-drag-region>
  <div class="switcher" use:clickOutside={() => (switcherOpen = false)}>
    <button type="button" class="connection" class:open={switcherOpen} onclick={() => (switcherOpen = !switcherOpen)}>
      {#if connection}
        <span class="env-dot tone-{environmentTone(connection.environment)}"></span>
        <span class="col grow">
          <span class="name truncate">{connection.name}</span>
          <span class="meta truncate">{ENVIRONMENT_LABEL[connection.environment].label}{connection.engineVersion ? ` · Camunda ${connection.engineVersion}` : ""}</span>
        </span>
      {:else}
        <span class="env-dot tone-neutral"></span>
        <span class="col grow">
          <span class="name">Нет подключения</span>
          <span class="meta">Добавьте окружение</span>
        </span>
      {/if}
      <Icon name="chevron" size={14} class="chevron {switcherOpen ? 'flipped' : ''}" />
    </button>
    {#if switcherOpen}
      <div class="menu card" transition:pop={{ duration: 150, start: 0.97 }}>
        {#each app.connections as item (item.id)}
          <button type="button" class="menu-item" class:active={item.id === app.selectedConnectionId} onclick={() => { app.selectConnection(item.id); switcherOpen = false; }}>
            <span class="env-dot tone-{environmentTone(item.environment)}"></span>
            <span class="col grow">
              <span class="name truncate">{item.name}</span>
              <span class="meta mono truncate">{item.restUrl}</span>
            </span>
            <EnvironmentBadge environment={item.environment} />
          </button>
        {:else}
          <div class="menu-empty muted">Подключений пока нет</div>
        {/each}
        <div class="divider"></div>
        <button type="button" class="menu-item add" onclick={() => { app.openConnectionDialog(); switcherOpen = false; }}>
          <Icon name="plus" size={14} />
          <span>Добавить подключение</span>
        </button>
      </div>
    {/if}
  </div>

  <button type="button" class="search" onclick={() => palette.show()}>
    <Icon name="search" size={15} />
    <span class="search-text">Найти заявку…</span>
    <span class="kbd">{isMac ? "⌘" : "Ctrl"}</span><span class="kbd">K</span>
  </button>

  <div class="actions">
    <button type="button" class="action" title="Обновить данные ({isMac ? '⌘' : 'Ctrl'} R)" onclick={() => palette.requestRefresh()}>
      <Icon name="refresh" size={15} />
    </button>
    <button type="button" class="action" disabled={!cockpitUrl} title="Открыть Cockpit в браузере" onclick={() => cockpitUrl && api.openInBrowser(cockpitUrl)}>
      <Icon name="external" size={15} />
      <span>Cockpit</span>
    </button>
  </div>
</header>

<style>
  .topbar {
    height: var(--topbar-height);
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 14px 0 10px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    transition: background-color var(--dur-slow) var(--ease-out), border-color var(--dur-slow) var(--ease-out);
  }
  .switcher {
    position: relative;
    width: 280px;
    flex: none;
  }
  .connection {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 36px;
    padding: 0 8px 0 10px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    transition: background-color var(--dur-fast), border-color var(--dur-fast);
  }
  .connection:hover,
  .connection.open {
    background: var(--surface-2);
    border-color: var(--border);
  }
  .env-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
  }
  .tone-danger { background: var(--danger); box-shadow: 0 0 0 3px color-mix(in srgb, var(--danger) 18%, transparent); }
  .tone-warning { background: var(--warning); box-shadow: 0 0 0 3px color-mix(in srgb, var(--warning) 18%, transparent); }
  .tone-development { background: var(--info); box-shadow: 0 0 0 3px color-mix(in srgb, var(--info) 18%, transparent); }
  .tone-neutral { background: var(--neutral); box-shadow: 0 0 0 3px color-mix(in srgb, var(--neutral) 18%, transparent); }
  .name {
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1.2;
  }
  .meta {
    font-size: 10px;
    color: var(--ink-3);
    line-height: 1.3;
  }
  :global(.chevron) {
    color: var(--ink-3);
    transition: transform var(--dur-base) var(--ease-spring);
  }
  :global(.chevron.flipped) {
    transform: rotate(180deg);
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    width: 360px;
    z-index: 60;
    padding: 6px;
    box-shadow: var(--shadow-3);
    transform-origin: top left;
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 9px;
    border: 0;
    border-radius: var(--radius-xs);
    background: transparent;
    text-align: left;
    transition: background-color var(--dur-fast);
  }
  .menu-item:hover {
    background: var(--surface-2);
  }
  .menu-item.active {
    background: var(--accent-soft);
  }
  .menu-item.add {
    color: var(--accent);
    font-weight: 500;
  }
  .menu-empty {
    padding: 10px;
    font-size: 12px;
  }
  .menu .divider {
    margin: 6px 0;
  }
  .search {
    flex: 1;
    max-width: 420px;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 8px 0 12px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    color: var(--ink-3);
    transition: border-color var(--dur-fast);
  }
  .search:hover {
    border-color: var(--border-strong);
  }
  .search-text {
    flex: 1;
    text-align: left;
    font-size: 12.5px;
  }
  .actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 10px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
    font-size: 12px;
    font-weight: 500;
    transition: background-color var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast);
  }
  .action:hover:not(:disabled) {
    background: var(--surface-2);
    border-color: var(--border);
    color: var(--ink);
  }
  .action:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
