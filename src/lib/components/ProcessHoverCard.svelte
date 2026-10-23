<script lang="ts">
  import StateBadge from "./StateBadge.svelte";
  import type { ProcessDefinitionSummary } from "../types";

  let { definition }: { definition: ProcessDefinitionSummary } = $props();
  const name = $derived(definition.name?.trim() ? definition.name : definition.key);
</script>

<div class="info">
  <div class="row">
    <div class="col grow">
      <span class="name">{name}</span>
      <span class="mono muted key">{definition.key}</span>
    </div>
    <StateBadge suspended={definition.suspended} />
  </div>
  {#if definition.description?.trim()}
    <p class="muted description clamp-2">{definition.description}</p>
  {/if}
  <div class="divider"></div>
  <div class="metrics">
    <div class="metric"><span class="eyebrow">Версия</span><span class="mono value primary">v{definition.version}{definition.versionTag ? ` · ${definition.versionTag}` : ""}</span></div>
    <div class="metric"><span class="eyebrow">Заявки</span><span class="mono value healthy">{definition.instances}</span></div>
    <div class="metric"><span class="eyebrow">Инциденты</span><span class="mono value" class:danger={definition.incidents > 0} class:healthy={definition.incidents === 0}>{definition.incidents}</span></div>
  </div>
  <div class="rows">
    <span class="muted">Deployment</span><span class="mono truncate">{definition.deploymentId ?? "—"}</span>
    <span class="muted">BPMN-ресурс</span><span class="mono truncate">{definition.resource ?? "—"}</span>
    <span class="muted">Тенант</span><span class="mono truncate">{definition.tenantId ?? "Без тенанта"}</span>
    <span class="muted">Хранение истории</span><span class="mono truncate">{definition.historyTimeToLive != null ? `${definition.historyTimeToLive} дней` : "Не задано"}</span>
  </div>
  <p class="faint tip">Нажмите на название, чтобы открыть процесс; на инциденты — чтобы открыть их список.</p>
</div>

<style>
  .info {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 13px;
  }
  .name {
    font-weight: 700;
    font-size: 14px;
  }
  .key {
    font-size: 10px;
  }
  .description {
    font-size: 10.5px;
  }
  .metrics {
    display: flex;
    gap: 18px;
  }
  .metric {
    display: flex;
    flex-direction: column;
  }
  .value {
    font-size: 12px;
    font-weight: 700;
  }
  .primary { color: var(--accent); }
  .healthy { color: var(--healthy); }
  .danger { color: var(--danger); }
  .rows {
    display: grid;
    grid-template-columns: 105px 1fr;
    gap: 3px 8px;
    font-size: 10px;
  }
  .tip {
    font-size: 9.5px;
  }
</style>
