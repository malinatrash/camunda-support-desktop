<script lang="ts">
  import { untrack } from "svelte";
  import Button from "../components/Button.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import PeriodPicker from "../components/PeriodPicker.svelte";
  import VirtualList from "../components/VirtualList.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Icon from "../components/Icon.svelte";
  import SegmentedTabs from "../components/SegmentedTabs.svelte";
  import { app } from "../state/app.svelte";
  import { palette } from "../state/palette.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { api, errorMessage } from "../api";
  import { groupIncidents, relativeTime, retryableCount, type IncidentGroup } from "../incidents";
  import { contains, definitionKeyOf, definitionVersionOf, incidentTypeLabel } from "../format";
  import { expand } from "../motion";
  import type { DashboardDateFilter, ProcessIncident } from "../types";

  interface VersionOption {
    definitionId: string;
    processKey: string;
    version: number;
    versionTag: string | null;
  }

  const connection = $derived(app.selectedConnection);
  const scopedDefinitionId = app.selectedIncidentProcessDefinitionId;
  const processDefinitionKey = scopedDefinitionId ? definitionKeyOf(scopedDefinitionId) : null;

  let incidents = $state<ProcessIncident[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let query = $state("");
  let selectedType = $state<string | null>(null);
  let refreshToken = $state(0);
  let appliedFilter = $state<DashboardDateFilter>(app.selectedIncidentDateFilter);
  let selectedDefinitionId = $state<string | null>(scopedDefinitionId);
  let versions = $state<VersionOption[]>([]);
  let view = $state<"groups" | "list">("groups");
  let selected = $state<Set<string>>(new Set());
  let pendingRetry = $state<ProcessIncident[] | null>(null);
  let retryBusy = $state(false);
  let expandedGroup = $state<string | null>(null);
  const production = $derived(connection?.environment === "Production");

  $effect(() => {
    palette.refreshSerial;
    untrack(() => (refreshToken += 1));
  });

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
  }

  function toggleAll() {
    selected = selected.size === filtered.length ? new Set() : new Set(filtered.map((incident) => incident.id));
  }

  async function confirmRetry() {
    const list = pendingRetry;
    if (!list || !connection) return;
    retryBusy = true;
    try {
      const result = await api.retryIncidents(connection, list, 3);
      toasts.success(`Повтор запущен: заданий ${result.jobs}, внешних задач ${result.externalTasks}${result.skipped ? `, пропущено ${result.skipped}` : ""}.`);
      pendingRetry = null;
      selected = new Set();
      refreshToken += 1;
    } catch (failure) {
      toasts.error(errorMessage(failure));
    } finally {
      retryBusy = false;
    }
  }

  $effect(() => {
    const current = connection;
    versions = [];
    if (!current || !processDefinitionKey) return;
    let cancelled = false;
    api
      .loadProcessDefinitionVersions(current, processDefinitionKey)
      .then((loaded) => {
        if (cancelled) return;
        versions = loaded.map((definition) => ({
          definitionId: definition.id,
          processKey: definition.key,
          version: definition.version,
          versionTag: definition.versionTag,
        }));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const current = connection;
    const definitionId = selectedDefinitionId;
    const filter = appliedFilter;
    refreshToken;
    if (!current) return;
    let cancelled = false;
    loading = true;
    error = null;
    api
      .loadIncidents(current, definitionId, filter, definitionId == null ? processDefinitionKey : null)
      .then((loaded) => {
        if (!cancelled) incidents = loaded;
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

  const types = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const incident of incidents) counts.set(incident.type, (counts.get(incident.type) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1]);
  });

  function toOption(definitionId: string): VersionOption | null {
    const key = definitionKeyOf(definitionId);
    const version = definitionVersionOf(definitionId);
    if (!key || version == null) return null;
    return { definitionId, processKey: key, version, versionTag: null };
  }

  const versionOptions = $derived.by(() => {
    const fromIncidents = new Map<string, VersionOption>();
    for (const incident of incidents) {
      if (!incident.processDefinitionId || fromIncidents.has(incident.processDefinitionId)) continue;
      const option = toOption(incident.processDefinitionId);
      if (option) fromIncidents.set(option.definitionId, option);
    }
    const options = versions.length > 0 ? versions : [...fromIncidents.values()];
    return [...options].sort((a, b) => b.version - a.version);
  });

  function optionLabel(option: VersionOption): string {
    const scoped = processDefinitionKey != null;
    return `${scoped ? "" : `${option.processKey} · `}Версия ${option.version}${option.versionTag ? ` · ${option.versionTag}` : ""}`;
  }

  const filtered = $derived(
    incidents.filter(
      (incident) =>
        (selectedType == null || incident.type === selectedType) &&
        (query.trim() === "" ||
          [incident.id, incident.message, incident.activityId, incident.failedActivityId, incident.processInstanceId, incident.processDefinitionId, incident.configuration].some(
            (value) => contains(value, query),
          )),
    ),
  );

  function groupProcessLabel(group: IncidentGroup): string {
    if (!group.processDefinitionId) return "Процесс неизвестен";
    const option = toOption(group.processDefinitionId);
    return option ? `${option.processKey} · v${option.version}` : group.processDefinitionId;
  }

  const groups = $derived(groupIncidents(filtered));
  const selectedIncidents = $derived(filtered.filter((incident) => selected.has(incident.id)));
  const retryableSelected = $derived(selectedIncidents.filter((incident) => (incident.type === "failedJob" || incident.type === "failedExternalTask") && incident.configuration));

  function processLabel(incident: ProcessIncident): string | null {
    if (!incident.processDefinitionId) return null;
    const option = toOption(incident.processDefinitionId);
    return option ? `процесс: ${option.processKey} · v${option.version}` : null;
  }
</script>

<div class="page">
  {#if !connection}
    <EmptyPanel title="Требуется подключение" description="Добавьте подключение Camunda, чтобы просматривать инциденты." actionLabel="Добавить подключение" onaction={() => app.openConnectionDialog()} />
  {:else}
    <header class="head">
      {#if processDefinitionKey}<span class="scope mono">{processDefinitionKey}</span>{/if}
      <PeriodPicker value={appliedFilter} onchange={(filter) => (appliedFilter = filter)} />
      <select class="select" value={selectedDefinitionId ?? ""} onchange={(event) => (selectedDefinitionId = event.currentTarget.value || null)}>
        <option value="">{processDefinitionKey == null ? "Все процессы" : "Все версии"}</option>
        {#each versionOptions as option (option.definitionId)}
          <option value={option.definitionId}>{optionLabel(option)}</option>
        {/each}
      </select>
      <span class="grow"></span>
      <input class="search" type="search" placeholder="Сообщение, заявка, кубик или ID" bind:value={query} />
      <button type="button" class="icon-button" class:spinning={loading} title="Обновить" onclick={() => (refreshToken += 1)}>
        <Icon name="refresh" size={15} />
      </button>
    </header>

    <div class="bar">
      <SegmentedTabs items={[{ id: "groups", label: "Группы", badge: groups.length }, { id: "list", label: "Список", badge: filtered.length }]} selected={view} onselect={(id) => (view = id)} size="sm" />
      <span class="sep"></span>
      <button type="button" class="type" class:active={selectedType == null} onclick={() => (selectedType = null)}>Все типы</button>
      {#each types.slice(0, 6) as [type, count] (type)}
        <button type="button" class="type" class:active={selectedType === type} onclick={() => (selectedType = type)}>{incidentTypeLabel(type)} <span class="counter">{count}</span></button>
      {/each}
      <span class="grow"></span>
      {#if view === "list"}
        <Button size="sm" variant="primary" disabled={retryableSelected.length === 0} onclick={() => (pendingRetry = retryableSelected)}>Повторить выбранные ({retryableSelected.length})</Button>
      {/if}
    </div>

    {#if error}
      <div transition:expand><div class="message message-danger selectable">{error}</div></div>
    {/if}

    {#if loading && incidents.length === 0}
      <div class="col gap">{#each [0, 1, 2, 3] as index (index)}<div class="shimmer skeleton"></div>{/each}</div>
    {:else if filtered.length === 0}
      <p class="calm">{incidents.length === 0 ? "Инцидентов нет" : "Ничего не найдено — измените фильтр"}</p>
    {:else if view === "groups"}
      <div class="groups scroll">
        {#each groups as group (group.key)}
          {@const open = expandedGroup === group.key}
          <div class="group" class:open>
            <div class="group-row" role="button" tabindex="0" onclick={() => (expandedGroup = open ? null : group.key)} onkeydown={(event) => event.key === "Enter" && (expandedGroup = open ? null : group.key)}>
              <span class="count"><span class="pill pill-danger">{group.count}</span></span>
              <span class="col grow">
                <span class="group-title truncate">{group.activityId ?? "Без кубика"}</span>
                <span class="group-message truncate selectable">{group.signature}</span>
                <span class="faint small truncate">{groupProcessLabel(group)} · {incidentTypeLabel(group.type)} · заявок {group.instanceIds.length} · {relativeTime(group.latestTimestamp)}</span>
              </span>
              <Button size="sm" variant="ghost" disabled={retryableCount(group) === 0} onclick={(event) => { event.stopPropagation(); pendingRetry = group.incidents; }}>Повторить ({retryableCount(group)})</Button>
            </div>
            {#if open}
              <div class="group-body" transition:expand={{ duration: 180 }}>
                {#each group.incidents.slice(0, 50) as incident (incident.id)}
                  <div class="row group-incident">
                    <span class="faint small mono">{incident.timestamp?.slice(0, 19).replace("T", " ") ?? "—"}</span>
                    <button type="button" class="link mono small truncate" disabled={!incident.processInstanceId} onclick={() => incident.processInstanceId && app.openProcessInstance(incident.processInstanceId)}>{incident.processInstanceId ?? "—"}</button>
                  </div>
                {/each}
                {#if group.incidents.length > 50}<span class="faint small">Показаны первые 50 из {group.incidents.length}</span>{/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <div class="list">
        <label class="pick-all small muted"><input type="checkbox" checked={selected.size === filtered.length && filtered.length > 0} onchange={toggleAll} /> Выбрать все</label>
        <VirtualList items={filtered} itemHeight={76} key={(incident) => incident.id}>
          {#snippet row(incident)}
            <div class="incident" class:selected={selected.has(incident.id)}>
              <input type="checkbox" checked={selected.has(incident.id)} onchange={() => toggle(incident.id)} />
              <div class="col grow">
                <span class="truncate selectable">{incident.message ?? "Camunda не вернула сообщение ошибки"}</span>
                <span class="mono faint small truncate selectable">
                  {[incidentTypeLabel(incident.type), incident.failedActivityId, processLabel(incident), incident.timestamp?.slice(0, 19).replace("T", " ")].filter(Boolean).join(" · ")}
                </span>
              </div>
              <Button size="sm" variant="ghost" disabled={!incident.processInstanceId} onclick={() => incident.processInstanceId && app.openProcessInstance(incident.processInstanceId)}>Заявка →</Button>
            </div>
          {/snippet}
        </VirtualList>
      </div>
    {/if}
  {/if}
</div>

<ConfirmDialog
  open={pendingRetry != null}
  description={pendingRetry ? `Выставить 3 попытки для ${pendingRetry.length} инцидентов? Camunda сразу начнёт повторное выполнение заданий и внешних задач.` : ""}
  production={production}
  busy={retryBusy}
  onconfirm={confirmRetry}
  oncancel={() => (pendingRetry = null)}
/>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 14px;
    height: 100%;
    min-height: 0;
    padding-bottom: 16px;
  }
  .head,
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: none;
  }
  .head :global(.picker) {
    margin-left: -9px;
  }
  .bar {
    border-bottom: 1px solid var(--border);
  }
  .scope {
    font-weight: 600;
  }
  .select,
  .search {
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    font-size: var(--text-md);
    outline: none;
  }
  .select {
    max-width: 260px;
  }
  .search {
    width: 280px;
  }
  .search:focus {
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  .icon-button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
  }
  .icon-button:hover {
    background: var(--surface-3);
    color: var(--ink);
  }
  .icon-button.spinning :global(svg) {
    animation: spin 0.9s linear infinite;
  }
  .sep {
    width: 1px;
    height: 14px;
    background: var(--border-strong);
  }
  .type {
    display: inline-flex;
    align-items: center;
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--ink-2);
    font-size: var(--text-md);
  }
  .type:hover {
    color: var(--ink);
    background: var(--row-hover);
  }
  .type.active {
    color: var(--ink);
    font-weight: 600;
    background: var(--surface-3);
  }
  .gap {
    gap: 12px;
  }
  .skeleton {
    height: 52px;
  }
  .calm {
    padding: 32px 12px;
    color: var(--ink-3);
  }
  .groups,
  .list {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }
  .groups {
    flex: 0 1 auto;
    min-height: 0;
  }
  .group {
    border-bottom: 1px solid var(--border);
  }
  .group:last-child {
    border-bottom: 0;
  }
  .group.open {
    background: var(--surface-2);
  }
  .group-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    cursor: pointer;
  }
  .group-row:hover {
    background: var(--row-hover);
  }
  .group-row:hover .group-title {
    color: var(--accent);
  }
  .count {
    min-width: 40px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    align-self: flex-start;
  }
  .group-title {
    font-weight: 500;
  }
  .group-message {
    color: var(--ink-2);
    font-size: var(--text-md);
  }
  .small {
    font-size: var(--text-sm);
  }
  .group-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 0 12px 12px 66px;
  }
  .group-incident {
    gap: 16px;
  }
  .list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .pick-all {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
    font-weight: 600;
  }
  .incident {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 76px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
  }
  .incident:hover {
    background: var(--row-hover);
  }
  .incident.selected {
    background: var(--row-selected);
  }
</style>
