<script lang="ts">
  import { untrack } from "svelte";
  import Button from "../components/Button.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import PeriodPicker from "../components/PeriodPicker.svelte";
  import StateBadge from "../components/StateBadge.svelte";
  import VirtualList from "../components/VirtualList.svelte";
  import { app } from "../state/app.svelte";
  import { palette } from "../state/palette.svelte";
  import { api, errorMessage } from "../api";
  import { processDefinitionCockpitUrl } from "../connection";
  import { definitionVersionOf, isLive, pluralize } from "../format";
  import { expand } from "../motion";
  import type { CamundaConnection, DashboardDateFilter, ProcessDefinitionDetails, ProcessDefinitionSummary } from "../types";

  let { connection, processDefinitionId }: { connection: CamundaConnection; processDefinitionId: string } = $props();

  let filter = $state<DashboardDateFilter>(app.selectedDashboardDateFilter);
  let versionId = $state<string | null>(untrack(() => processDefinitionId));
  let versions = $state<ProcessDefinitionSummary[]>([]);
  let details = $state<ProcessDefinitionDetails | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let metadataVisible = $state(false);
  let refreshToken = $state(0);

  const live = $derived(isLive(filter));
  const definitionKey = $derived(details?.definition.key ?? processDefinitionId.split(":")[0]);

  $effect(() => {
    let cancelled = false;
    api
      .loadProcessDefinitionVersions(connection, processDefinitionId.split(":")[0])
      .then((loaded) => {
        if (!cancelled) versions = loaded;
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    refreshToken;
    palette.refreshSerial;
    const id = versionId ?? processDefinitionId;
    const allVersions = versionId == null;
    const period = filter;
    let cancelled = false;
    loading = true;
    error = null;
    api
      .loadProcessDefinitionDetails(connection, id, period, allVersions)
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

  const cockpitUrl = $derived(processDefinitionCockpitUrl(connection, versionId ?? processDefinitionId));
  const withIncidents = $derived(details ? details.instances.filter((item) => item.incidentCount > 0).length : 0);
  const total = $derived(details?.instances.length ?? 0);

  function selectFilter(next: DashboardDateFilter) {
    filter = next;
    app.selectedDashboardDateFilter = next;
  }
</script>

<div class="page">
  <header class="head">
    <button type="button" class="back" onclick={() => app.backFromProcessDefinition()} title="Назад">
      <Icon name="arrowLeft" size={16} />
    </button>
    <div class="col grow">
      <h1 class="truncate">{details?.definition.name?.trim() || definitionKey}</h1>
      <span class="faint mono meta selectable">{definitionKey}{details?.definition.tenantId ? ` · ${details.definition.tenantId}` : ""}</span>
    </div>
    <select class="select" value={versionId ?? ""} onchange={(event) => (versionId = event.currentTarget.value || null)}>
      <option value="">Все версии</option>
      {#each versions as version (version.id)}
        <option value={version.id}>v{version.version}{version.versionTag ? ` · ${version.versionTag}` : ""}</option>
      {/each}
    </select>
    <Button size="sm" variant="ghost" onclick={() => (metadataVisible = true)} disabled={!details}>Метаданные</Button>
    <Button size="sm" variant="ghost" disabled={!cockpitUrl} onclick={() => cockpitUrl && api.openInBrowser(cockpitUrl)}>Cockpit ↗</Button>
  </header>

  <div class="toolbar">
    <PeriodPicker value={filter} onchange={selectFilter} />
    <span class="grow"></span>
    {#if details}
      <span class="muted">
        {total}{total >= 500 ? "+" : ""} {pluralize(total, "заявка", "заявки", "заявок")}{withIncidents > 0 ? " · " : ""}
        {#if withIncidents > 0}<span class="danger">{withIncidents} с инцидентами</span>{/if}
      </span>
    {/if}
    <button type="button" class="icon-button" class:spinning={loading} title="Обновить" onclick={() => (refreshToken += 1)}>
      <Icon name="refresh" size={15} />
    </button>
  </div>

  {#if error}
    <div transition:expand><div class="message message-danger selectable">{error}</div></div>
  {/if}

  {#if !details}
    {#if !error}<div class="shimmer list-skeleton"></div>{/if}
  {:else if details.instances.length === 0}
    <EmptyPanel
      title={live ? "Запущенных заявок нет" : "За период заявок нет"}
      description={versionId ? "Попробуйте выбрать «Все версии»: заявки могут работать на старой версии процесса." : "Camunda не вернула заявок для этого процесса."}
      compact
    />
  {:else}
    <div class="table">
      <div class="table-head">
        <span class="cell" style:flex="0.9">Состояние</span>
        <span class="cell" style:flex="1.4">Бизнес-ключ</span>
        <span class="cell" style:flex="1.6">ID заявки</span>
        <span class="cell" style:flex="0.5">Версия</span>
        <span class="cell num" style:flex="0.6">Инциденты</span>
        <span class="cell" style:flex="1">{live ? "Тенант" : "Старт"}</span>
      </div>
      <div class="table-body">
        <VirtualList items={details.instances} itemHeight={40} key={(item) => item.instance.id}>
          {#snippet row(item)}
            <div
              class="table-row instance-row"
              class:clickable={!item.instance.ended}
              role="button"
              tabindex="0"
              onclick={() => !item.instance.ended && app.openProcessInstance(item.instance.id)}
              onkeydown={(event) => event.key === "Enter" && !item.instance.ended && app.openProcessInstance(item.instance.id)}
            >
              <span class="cell" style:flex="0.9"><StateBadge suspended={item.instance.suspended} ended={item.instance.ended} /></span>
              <span class="cell truncate selectable" style:flex="1.4">{item.instance.businessKey ?? "—"}</span>
              <span class="cell mono small truncate selectable" style:flex="1.6">{item.instance.id}</span>
              <span class="cell mono small muted" style:flex="0.5">v{definitionVersionOf(item.instance.definitionId) ?? "?"}</span>
              <span class="cell num mono" style:flex="0.6">{#if item.incidentCount > 0}<span class="pill pill-danger">{item.incidentCount}</span>{:else}<span class="faint">0</span>{/if}</span>
              <span class="cell muted small truncate" style:flex="1">{live ? (item.instance.tenantId ?? "—") : (item.instance.startTime?.slice(0, 16).replace("T", " ") ?? "—")}</span>
            </div>
          {/snippet}
        </VirtualList>
      </div>
    </div>
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
    gap: 16px;
    height: 100%;
    padding-bottom: 16px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .back,
  .icon-button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    flex: none;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
  }
  .back:hover,
  .icon-button:hover {
    background: var(--surface-3);
    color: var(--ink);
  }
  .icon-button.spinning :global(svg) {
    animation: spin 0.9s linear infinite;
  }
  h1 {
    font-size: var(--text-xl);
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .meta,
  .small {
    font-size: var(--text-sm);
  }
  .select {
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    font-size: var(--text-md);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .danger {
    color: var(--danger);
  }
  .list-skeleton {
    flex: 1;
  }
  .table {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .table-body {
    flex: 1;
    min-height: 0;
  }
  .instance-row {
    height: 40px;
  }
  .num {
    text-align: right;
    padding-right: 24px;
  }
  .metadata {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 7px 12px;
    font-size: var(--text-sm);
  }
</style>
