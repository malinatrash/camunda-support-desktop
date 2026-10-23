<script lang="ts">
  import Button from "../components/Button.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import Modal from "../components/Modal.svelte";
  import StateBadge from "../components/StateBadge.svelte";
  import VirtualList from "../components/VirtualList.svelte";
  import { app } from "../state/app.svelte";
  import { api, errorMessage } from "../api";
  import { processDefinitionCockpitUrl } from "../connection";
  import { instancesTitle, isLive } from "../format";
  import { expand, rise } from "../motion";
  import type { CamundaConnection, ProcessDefinitionDetails } from "../types";

  let { connection, processDefinitionId }: { connection: CamundaConnection; processDefinitionId: string } = $props();

  const dateFilter = app.selectedDashboardDateFilter;
  const live = isLive(dateFilter);

  let details = $state<ProcessDefinitionDetails | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let metadataVisible = $state(false);
  let refreshToken = $state(0);

  $effect(() => {
    refreshToken;
    let cancelled = false;
    loading = true;
    error = null;
    api
      .loadProcessDefinitionDetails(connection, processDefinitionId, dateFilter)
      .then((loaded) => {
        if (!cancelled) details = loaded;
      })
      .catch((failure) => {
        if (!cancelled) error = errorMessage(failure);
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  const cockpitUrl = $derived(processDefinitionCockpitUrl(connection, processDefinitionId));
  const withIncidents = $derived(details ? details.instances.filter((item) => item.incidentCount > 0).length : 0);
</script>

<div class="page">
  <div class="row head">
    <Button onclick={() => app.backFromProcessDefinition()}>← Назад</Button>
    <div class="col grow">
      <h1 class="title">{details?.definition.name?.trim() || details?.definition.key || "Процесс"}</h1>
      <span class="mono muted small selectable">{processDefinitionId}</span>
    </div>
    <Button loading={loading} onclick={() => (refreshToken += 1)}>{loading ? "Обновляем…" : "Обновить"}</Button>
    <Button disabled={!cockpitUrl} onclick={() => cockpitUrl && api.openInBrowser(cockpitUrl)}>Открыть в браузере ↗</Button>
  </div>

  {#if error}
    <div transition:expand>
      <EmptyPanel title="Не удалось загрузить процесс" description={error} actionLabel="Повторить" onaction={() => (refreshToken += 1)} tone="danger" compact />
    </div>
  {/if}

  {#if !details}
    {#if !error}
      <div class="metrics">
        {#each [0, 1, 2, 3] as index (index)}<div class="card shimmer skeleton-metric"></div>{/each}
      </div>
      <div class="card shimmer skeleton-list"></div>
    {/if}
  {:else}
    <div class="metrics" in:rise>
      <div class="card metric"><span class="eyebrow">Версия</span><span class="mono value">v{details.definition.version}</span></div>
      <div class="card metric"><span class="eyebrow">{live ? "Запущено сейчас" : "Стартовало за период"}</span><span class="mono value">{details.instances.length}</span></div>
      <div class="card metric"><span class="eyebrow">С инцидентами</span><span class="mono value" class:danger={withIncidents > 0}>{withIncidents}</span></div>
      <div class="card metric"><span class="eyebrow">Тенант</span><span class="mono value truncate">{details.definition.tenantId ?? "—"}</span></div>
    </div>
    <div class="row">
      <Button size="sm" onclick={() => (metadataVisible = true)}>Подробнее о процессе</Button>
      <span class="list-title">{instancesTitle(dateFilter)}</span>
    </div>
    {#if details.instances.length === 0}
      <EmptyPanel
        title={live ? "Запущенных экземпляров нет" : "За период заявок нет"}
        description={live ? "Для этой версии процесса Camunda не вернула запущенные экземпляры." : "За выбранный период этот процесс не запускался."}
        compact
      />
    {:else}
      <div class="card table" in:rise={{ delay: 40 }}>
        <div class="table-head">
          <span class="cell" style:flex="0.8">Состояние</span>
          <span class="cell" style:flex="0.7">Инциденты</span>
          <span class="cell" style:flex="1.4">Бизнес-ключ</span>
          <span class="cell" style:flex="1.5">ID экземпляра</span>
          <span class="cell" style:flex="1.1">{live ? "Тенант" : "Время старта"}</span>
          <span class="cell" style:flex="0.7"></span>
        </div>
        <div class="table-body">
          <VirtualList items={details.instances} itemHeight={46} key={(item) => item.instance.id}>
            {#snippet row(item)}
              <div class="table-row instance-row" class:clickable={!item.instance.ended} role="button" tabindex="0" onclick={() => !item.instance.ended && app.openProcessInstance(item.instance.id)} onkeydown={(event) => event.key === "Enter" && !item.instance.ended && app.openProcessInstance(item.instance.id)}>
                <span class="cell" style:flex="0.8"><StateBadge suspended={item.instance.suspended} ended={item.instance.ended} /></span>
                <span class="cell mono" class:danger={item.incidentCount > 0} class:muted={item.incidentCount === 0} class:bold={item.incidentCount > 0} style:flex="0.7">{item.incidentCount}</span>
                <span class="cell truncate selectable" style:flex="1.4">{item.instance.businessKey ?? "—"}</span>
                <span class="cell mono small truncate selectable" style:flex="1.5">{item.instance.id}</span>
                <span class="cell muted small truncate" style:flex="1.1">{live ? (item.instance.tenantId ?? "—") : (item.instance.startTime ?? "—")}</span>
                <span class="cell right" style:flex="0.7">
                  <Button size="sm" variant={item.instance.ended ? "ghost" : "primary"} disabled={item.instance.ended} onclick={(event) => { event.stopPropagation(); app.openProcessInstance(item.instance.id); }}>
                    {item.instance.ended ? "Завершена" : "Подробнее"}
                  </Button>
                </span>
              </div>
            {/snippet}
          </VirtualList>
        </div>
      </div>
    {/if}
  {/if}
</div>

<Modal open={metadataVisible} title="Метаданные процесса" width="640px" onclose={() => (metadataVisible = false)}>
  {#if details}
    <div class="metadata selectable">
      {#each details.metadata as field (field.label)}
        <span class="muted">{field.label}</span>
        <span class="mono">{field.value}</span>
      {/each}
    </div>
  {/if}
  {#snippet footer()}
    <Button variant="primary" onclick={() => (metadataVisible = false)}>Закрыть</Button>
  {/snippet}
</Modal>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    height: 100%;
  }
  .head {
    gap: 10px;
  }
  .title {
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }
  .small {
    font-size: 10.5px;
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }
  .metric {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 9px 12px;
  }
  .value {
    font-size: 17px;
    font-weight: 600;
  }
  .danger {
    color: var(--danger);
  }
  .bold {
    font-weight: 700;
  }
  .skeleton-metric {
    height: 54px;
  }
  .skeleton-list {
    flex: 1;
  }
  .list-title {
    font-size: 15px;
    font-weight: 650;
  }
  .table {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .table-body {
    flex: 1;
    min-height: 0;
  }
  .instance-row {
    height: 46px;
  }
  .right {
    display: flex;
    justify-content: flex-end;
    padding-right: 0;
  }
  .metadata {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 7px 12px;
    font-size: 11px;
  }
</style>
