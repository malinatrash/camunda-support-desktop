<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../state/app.svelte";
  import { theme } from "../state/theme.svelte";
  import { updates } from "../state/updates.svelte";
  import { SIDEBAR_DESTINATIONS, type AppDestination } from "../types";

  const parentDestination = $derived<AppDestination>(
    app.destination === "ProcessDefinition" || app.destination === "ProcessInstance" ? "Overview" : app.destination,
  );
  const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
  const modifier = isMac ? "⌘" : "Ctrl";

  let buttons = $state<Record<string, HTMLButtonElement>>({});
  let indicator = $state({ top: 0, ready: false });

  $effect(() => {
    const button = buttons[parentDestination];
    if (button) indicator = { top: button.offsetTop, ready: true };
  });
</script>

<nav class="rail" data-tauri-drag-region>
  <div class="inset" data-tauri-drag-region></div>
  <div class="logo" title="Поддержка Camunda">
    <span class="logo-dot"></span>
  </div>

  <div class="items">
    <span class="indicator" class:ready={indicator.ready} style:transform="translateY({indicator.top}px)" aria-hidden="true"></span>
    {#each SIDEBAR_DESTINATIONS as item (item.id)}
      {@const active = parentDestination === item.id}
      <button type="button" class="item" class:active bind:this={buttons[item.id]} onclick={() => app.navigate(item.id)} title="{item.title} · {modifier} {item.shortcut}">
        <Icon name={item.icon} size={18} stroke={1.7} />
        <span class="label">{item.title}</span>
      </button>
    {/each}
  </div>

  <div class="grow"></div>

  <button type="button" class="item quiet" onclick={() => theme.set(theme.mode === "Dark" ? "Light" : "Dark")} title={theme.mode === "Dark" ? "Светлая тема" : "Тёмная тема"}>
    <Icon name={theme.mode === "Dark" ? "sun" : "moon"} size={17} />
  </button>
  <span class="version" title="Сборка {updates.info.build}">{updates.info.version}</span>
</nav>

<style>
  .rail {
    width: var(--rail-width);
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 10px 0;
    background: var(--rail);
    color: var(--rail-ink);
    border-right: 1px solid var(--border);
    min-height: 0;
    transition: background-color var(--dur-slow) var(--ease-out), border-color var(--dur-slow) var(--ease-out);
  }
  .inset {
    height: var(--titlebar-inset);
    flex: none;
    width: 100%;
  }
  .logo {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    background: var(--accent);
    margin: 4px 0 14px;
  }
  .logo-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #fff;
  }
  .items {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    align-items: center;
  }
  .indicator {
    position: absolute;
    left: 4px;
    top: 0;
    width: 64px;
    height: 50px;
    border-radius: 8px;
    background: var(--rail-active);
    opacity: 0;
  }
  .indicator.ready {
    opacity: 1;
    transition: transform var(--dur-slow) var(--ease-spring);
  }
  .item {
    position: relative;
    z-index: 1;
    width: 64px;
    height: 50px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--rail-ink);
    transition: color var(--dur-fast) var(--ease-out), background-color var(--dur-fast);
  }
  .item.quiet {
    height: 36px;
  }
  .item:hover {
    color: var(--rail-ink-strong);
  }
  .item:hover:not(.active) {
    background: var(--surface-3);
  }
  .item.active {
    color: var(--accent);
  }
  .label {
    font-size: 9.5px;
    white-space: nowrap;
    font-weight: 500;
    line-height: 1;
  }
  .version {
    font-size: 10px;
    color: var(--ink-3);
    margin-top: 6px;
  }
</style>
