<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import TopBar from "./lib/components/TopBar.svelte";
  import TabStrip from "./lib/components/TabStrip.svelte";
  import ConnectionDialog from "./lib/components/ConnectionDialog.svelte";
  import CommandPalette from "./lib/components/CommandPalette.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import Modal from "./lib/components/Modal.svelte";
  import Button from "./lib/components/Button.svelte";
  import OverviewScreen from "./lib/screens/OverviewScreen.svelte";
  import SearchScreen from "./lib/screens/SearchScreen.svelte";
  import IncidentsScreen from "./lib/screens/IncidentsScreen.svelte";
  import SettingsScreen from "./lib/screens/SettingsScreen.svelte";
  import ProcessDefinitionScreen from "./lib/screens/ProcessDefinitionScreen.svelte";
  import ProcessInstanceScreen from "./lib/screens/ProcessInstanceScreen.svelte";
  import { app, type DeepLinkNavigationResult } from "./lib/state/app.svelte";
  import { theme } from "./lib/state/theme.svelte";
  import { updates } from "./lib/state/updates.svelte";
  import { palette } from "./lib/state/palette.svelte";
  import { api, inTauri } from "./lib/api";
  import { expand, screenIn, screenOut } from "./lib/motion";
  import { DESTINATIONS, type DeepLinkEvent } from "./lib/types";

  let deepLinkError = $state<{ message: string; canOpenConnections: boolean } | null>(null);
  let deepLinkChoice = $state<Extract<DeepLinkNavigationResult, { kind: "choiceRequired" }> | null>(null);

  const screenKey = $derived(
    `${app.destination}:${app.selectedConnectionId ?? ""}:${app.selectedProcessDefinitionId ?? ""}:${app.selectedProcessInstanceId ?? ""}`,
  );

  function handleDeepLink(event: DeepLinkEvent) {
    if (event.kind === "invalid") {
      deepLinkError = { message: event.message, canOpenConnections: false };
      return;
    }
    const result = app.openDeepLink(event.link);
    if (result.kind === "opened") {
      deepLinkError = null;
      deepLinkChoice = null;
    } else if (result.kind === "connectionNotFound") {
      deepLinkError = { message: `Не найдено сохранённое подключение для ${result.link.origin}.`, canOpenConnections: true };
    } else {
      deepLinkChoice = result;
    }
  }

  async function drainDeepLinks() {
    try {
      (await api.takePendingDeepLinks()).forEach(handleDeepLink);
    } catch {
      // Нет ожидающих ссылок.
    }
  }

  async function openLink(raw: string) {
    handleDeepLink(await api.parseDeepLink(raw));
  }

  // Глобальные горячие клавиши: ⌘K палитра, ⌘1–3 разделы, ⌘, настройки, ⌘W закрыть вкладку, ⌘R обновить.
  function onKeydown(event: KeyboardEvent) {
    const meta = event.metaKey || event.ctrlKey;
    if (!meta) {
      if (event.key === "Escape" && palette.open) palette.hide();
      return;
    }
    const key = event.key.toLowerCase();
    if (key === "k") {
      event.preventDefault();
      palette.toggle();
    } else if (key === "r") {
      event.preventDefault();
      palette.requestRefresh();
    } else if (key === "w" && app.activeProcessTabId && app.destination === "ProcessInstance") {
      event.preventDefault();
      app.closeProcessTab(app.activeProcessTabId);
    } else {
      const destination = DESTINATIONS.find((item) => item.shortcut === event.key);
      if (destination) {
        event.preventDefault();
        app.navigate(destination.id);
      }
    }
  }

  onMount(() => {
    document.documentElement.dataset.platform = navigator.userAgent.includes("Mac") ? "macos" : "other";
    document.documentElement.dataset.shell = inTauri ? "tauri" : "browser";
    void theme.load();
    void app.load().then(drainDeepLinks);
    void updates.start();
    const unlisten = inTauri ? listen("deep-link", drainDeepLinks) : Promise.resolve(() => undefined);
    return () => {
      void unlisten.then((stop) => stop());
      updates.stop();
    };
  });

  $effect(() => {
    if (inTauri) void getCurrentWindow().setTitle(`Поддержка Camunda · версия ${updates.info.version} · сборка ${updates.info.build}`);
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app-shell">
  <TopBar />
  {#if app.openProcessTabs.length > 0}
    <div transition:expand={{ duration: 160 }}>
      <TabStrip />
    </div>
  {/if}
  <main class="screen-host">
      {#if app.ready}
        {#key screenKey}
          <div class="screen" in:screenIn out:screenOut>
            {#if app.destination === "Overview"}
              <OverviewScreen />
            {:else if app.destination === "Processes"}
              <SearchScreen />
            {:else if app.destination === "Incidents"}
              <IncidentsScreen />
            {:else if app.destination === "Settings"}
              <SettingsScreen />
            {:else if app.destination === "ProcessDefinition" && app.selectedConnection && app.selectedProcessDefinitionId}
              <ProcessDefinitionScreen connection={app.selectedConnection} processDefinitionId={app.selectedProcessDefinitionId} />
            {:else if app.destination === "ProcessInstance" && app.selectedConnection && app.selectedProcessInstanceId}
              <ProcessInstanceScreen connection={app.selectedConnection} processInstanceId={app.selectedProcessInstanceId} />
            {/if}
          </div>
        {/key}
      {/if}
  </main>
</div>

<CommandPalette onOpenLink={openLink} />
<ConnectionDialog />
<Toasts />

<Modal open={deepLinkError != null} title="Не удалось открыть ссылку" width="460px" onclose={() => (deepLinkError = null)}>
  <p class="selectable">{deepLinkError?.message}</p>
  {#snippet footer()}
    {#if deepLinkError?.canOpenConnections}
      <Button variant="ghost" onclick={() => (deepLinkError = null)}>Закрыть</Button>
      <Button variant="primary" onclick={() => { deepLinkError = null; app.navigate("Settings"); }}>Подключения</Button>
    {:else}
      <Button variant="primary" onclick={() => (deepLinkError = null)}>Закрыть</Button>
    {/if}
  {/snippet}
</Modal>

<Modal open={deepLinkChoice != null} title="Выберите подключение" width="460px" onclose={() => (deepLinkChoice = null)}>
  {#if deepLinkChoice}
    <p class="muted">Для домена {deepLinkChoice.link.origin} найдено несколько подключений.</p>
    <div class="choice-list">
      {#each deepLinkChoice.connections as connection (connection.id)}
        <Button block onclick={() => { if (deepLinkChoice) app.openDeepLinkWithConnection(connection.id, deepLinkChoice.link); deepLinkChoice = null; }}>
          {connection.name}
        </Button>
      {/each}
    </div>
  {/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (deepLinkChoice = null)}>Отмена</Button>
  {/snippet}
</Modal>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--canvas);
  }
  .screen-host {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    grid-template-columns: minmax(0, 1fr);
    position: relative;
  }
  .screen {
    grid-area: 1 / 1;
    height: 100%;
    overflow: hidden;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 20px 24px 0;
    will-change: transform, opacity;
  }
  .screen > :global(*) {
    flex: 1 1 auto;
    min-height: 0;
  }
  .choice-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
