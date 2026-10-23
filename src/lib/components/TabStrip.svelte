<script lang="ts">
  import { flip } from "svelte/animate";
  import Icon from "./Icon.svelte";
  import { app } from "../state/app.svelte";
  import { flipOptions, pop } from "../motion";
</script>

<div class="strip">
  <div class="tabs scroll">
    {#each app.openProcessTabs as tab (tab.id)}
      {@const active = tab.id === app.activeProcessTabId && app.destination === "ProcessInstance"}
      <div class="tab" class:active animate:flip={flipOptions} in:pop={{ duration: 180 }} out:pop={{ duration: 120 }} role="presentation">
        <button type="button" class="tab-main" onclick={() => app.activateProcessTab(tab.id)}>
          <span class="dot"></span>
          <span class="tab-title truncate" class:mono={!tab.businessKey}>{tab.businessKey ?? tab.processInstanceId}</span>
          <span class="tab-meta truncate">{[tab.processDefinitionKey, tab.connectionName].filter(Boolean).join(" · ")}</span>
        </button>
        <button type="button" class="tab-close" onclick={() => app.closeProcessTab(tab.id)} aria-label="Закрыть вкладку">
          <Icon name="x" size={12} stroke={2} />
        </button>
      </div>
    {/each}
  </div>
</div>

<style>
  .strip {
    flex: none;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    padding: 0 10px;
    transition: background-color var(--dur-slow) var(--ease-out), border-color var(--dur-slow) var(--ease-out);
  }
  .tabs {
    display: flex;
    gap: 2px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    height: 38px;
    align-items: flex-end;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    width: 230px;
    flex: none;
    height: 32px;
    padding-right: 4px;
    border-radius: 8px 8px 0 0;
    border: 1px solid transparent;
    border-bottom: 0;
    color: var(--ink-2);
    transition: background-color var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast);
  }
  .tab:hover {
    background: var(--surface-2);
  }
  .tab.active {
    background: var(--canvas);
    border-color: var(--border);
    color: var(--ink);
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 1px;
    background: var(--canvas);
  }
  .tab-main {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: auto minmax(0, auto) minmax(0, 1fr);
    align-items: center;
    gap: 7px;
    height: 100%;
    padding: 0 8px 0 10px;
    border: 0;
    background: transparent;
    text-align: left;
    color: inherit;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ink-3);
  }
  .active .dot {
    background: var(--accent);
  }
  .tab-title {
    font-size: 11.5px;
    font-weight: 600;
    max-width: 120px;
  }
  .tab-meta {
    font-size: 10px;
    color: var(--ink-3);
  }
  .tab-close {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--ink-3);
    transition: background-color var(--dur-fast), color var(--dur-fast);
  }
  .tab-close:hover {
    background: var(--surface-3);
    color: var(--ink);
  }
</style>
