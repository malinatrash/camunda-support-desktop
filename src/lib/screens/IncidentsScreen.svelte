<script lang="ts">
  import Button from "../components/Button.svelte";
  import Combobox, { type ComboboxItem } from "../components/Combobox.svelte";
  import DateFilterBar from "../components/DateFilterBar.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import SectionTitle from "../components/SectionTitle.svelte";
  import TextField from "../components/TextField.svelte";
  import VirtualList from "../components/VirtualList.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Icon from "../components/Icon.svelte";
  import SegmentedTabs from "../components/SegmentedTabs.svelte";
  import { app } from "../state/app.svelte";
  import { palette } from "../state/palette.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { api, errorMessage } from "../api";
  import { groupIncidents, relativeTime, retryableCount, type IncidentGroup } from "../incidents";
  import { contains, DATE_PRESET_LABEL, definitionKeyOf, definitionVersionOf, incidentTypeLabel, isLive } from "../format";
  import { expand, rise } from "../motion";
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
  let versionsLoading = $state(false);
  let view = $state<"groups" | "list">("groups");
  let selected = $state<Set<string>>(new Set());
  let pendingRetry = $state<ProcessIncident[] | null>(null);
  let retryBusy = $state(false);
  let expandedGroup = $state<string | null>(null);
  const production = $derived(connection?.environment === "Production");

  $effect(() => {
    palette.refreshSerial;
    refreshToken += 1;
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
    versionsLoading = true;
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
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled) versionsLoading = false;
      });
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

  const versionItems = $derived<ComboboxItem[]>(
    versionOptions.map((option) => ({ id: option.definitionId, title: optionLabel(option), subtitle: option.definitionId })),
  );

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
  <SectionTitle
    title={scopedDefinitionId == null ? "Инциденты" : "Инциденты процесса"}
    description={`${isLive(appliedFilter) ? "Открытые инциденты Camunda" : `История инцидентов: ${DATE_PRESET_LABEL[appliedFilter.preset].toLowerCase()}`}${processDefinitionKey ? ` · ${processDefinitionKey}` : ""}`}
  >
    {#if connection}
      <Button loading={loading} onclick={() => (refreshToken += 1)}>{loading ? "Обновляем…" : "Обновить"}</Button>
    {/if}
  </SectionTitle>

  {#if !connection}
    <EmptyPanel title="Требуется подключение" description="Добавьте подключение Camunda, чтобы просматривать инциденты." actionLabel="Добавить подключение" onaction={() => app.openConnectionDialog()} />
  {:else}
    {#if error}
      <div transition:expand>
        <EmptyPanel title="Не удалось загрузить инциденты" description={error} actionLabel="Повторить" onaction={() => (refreshToken += 1)} tone="danger" compact />
      </div>
    {/if}

    <div class="card filters" in:rise>
      <div class="row filters-top">
        <div class="grow">
          <DateFilterBar applied={appliedFilter} onapply={(filter) => (appliedFilter = filter)} label="Период" liveLabel="Открытые" />
        </div>
        <div class="version">
          <Combobox
            label="Версия"
            items={versionItems}
            selectedId={selectedDefinitionId}
            onselect={(item) => (selectedDefinitionId = item.id)}
            placeholder={processDefinitionKey == null ? "Все процессы и версии" : "Все версии процесса"}
            loading={versionsLoading}
            loadingText="Загружаем версии…"
            allowClear
            clearLabel={processDefinitionKey == null ? "Все процессы и версии" : "Все версии процесса"}
            onclear={() => (selectedDefinitionId = null)}
            width="380px"
          />
        </div>
      </div>
      <div class="row">
        <span class="bold">Всего: {incidents.length}</span>
        <TextField class="grow" size="sm" placeholder="Сообщение, заявка, кубик или ID" bind:value={query} type="search" />
        <SegmentedTabs items={[{ id: "groups", label: "Группы", badge: groups.length }, { id: "list", label: "Список", badge: filtered.length }]} selected={view} onselect={(id) => (view = id)} size="sm" />
        {#if view === "list"}
          <Button size="sm" variant="primary" disabled={retryableSelected.length === 0} onclick={() => (pendingRetry = retryableSelected)}>
            <Icon name="play" size={12} /> Повторить выбранные ({retryableSelected.length})
          </Button>
        {/if}
      </div>
      <div class="row types">
        <button type="button" class="chip" class:active={selectedType == null} onclick={() => (selectedType = null)}>Все · {incidents.length}</button>
        {#each types.slice(0, 6) as [type, count] (type)}
          <button type="button" class="chip" class:active={selectedType === type} onclick={() => (selectedType = type)}>{incidentTypeLabel(type)} · {count}</button>
        {/each}
      </div>
    </div>

    {#if loading && incidents.length === 0}
      <div class="col gap">
        {#each [0, 1, 2] as index (index)}<div class="card shimmer skeleton"></div>{/each}
      </div>
    {:else if filtered.length === 0}
      <EmptyPanel title="Инциденты не найдены" description="Измените фильтр или обновите данные." compact />
    {:else if view === "groups"}
      <div class="groups scroll" in:rise={{ delay: 40 }}>
        {#each groups as group (group.key)}
          {@const open = expandedGroup === group.key}
          <div class="card group" class:open>
            <button type="button" class="group-row" onclick={() => (expandedGroup = open ? null : group.key)}>
              <span class="count mono">{group.count}</span>
              <span class="col grow">
                <span class="row">
                  <span class="group-title truncate">{group.activityId ?? "Без кубика"}</span>
                  <span class="pill pill-danger">{incidentTypeLabel(group.type)}</span>
                  <span class="faint small">{groupProcessLabel(group)}</span>
                </span>
                <span class="group-message truncate selectable">{group.signature}</span>
                <span class="faint small">заявок: {group.instanceIds.length} · последний {relativeTime(group.latestTimestamp)}</span>
              </span>
              <Button size="sm" variant="primary" disabled={retryableCount(group) === 0} onclick={(event) => { event.stopPropagation(); pendingRetry = group.incidents; }}>
                <Icon name="play" size={12} /> Повторить ({retryableCount(group)})
              </Button>
              <Icon name="chevron" size={14} class="chevron {open ? 'flipped' : ''}" />
            </button>
            {#if open}
              <div class="group-body" transition:expand={{ duration: 200 }}>
                {#each group.incidents.slice(0, 50) as incident (incident.id)}
                  <div class="row group-incident">
                    <span class="faint small">{incident.timestamp ?? "—"}</span>
                    <span class="mono small truncate grow selectable">{incident.processInstanceId ?? "—"}</span>
                    <button type="button" class="link small" disabled={!incident.processInstanceId} onclick={() => incident.processInstanceId && app.openProcessInstance(incident.processInstanceId)}>Открыть заявку</button>
                  </div>
                {/each}
                {#if group.incidents.length > 50}<span class="faint small">Показаны первые 50 из {group.incidents.length}</span>{/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <div class="list" in:rise={{ delay: 40 }}>
        <div class="row select-all">
          <label class="pick-all"><input type="checkbox" checked={selected.size === filtered.length && filtered.length > 0} onchange={toggleAll} /> Выбрать все ({filtered.length})</label>
        </div>
        <VirtualList items={filtered} itemHeight={118} key={(incident) => incident.id}>
          {#snippet row(incident)}
            <div class="incident-slot">
              <div class="card incident row" class:selected={selected.has(incident.id)}>
                <label class="pick">
                  <input type="checkbox" checked={selected.has(incident.id)} onchange={() => toggle(incident.id)} />
                </label>
                <div class="col grow gap5">
                  <div class="row">
                    <span class="pill pill-danger">{incidentTypeLabel(incident.type)}</span>
                    <span class="faint small">{incident.timestamp ?? "Время неизвестно"}</span>
                  </div>
                  <span class="message-text clamp-2 selectable">{incident.message ?? "Camunda не вернула сообщение ошибки"}</span>
                  <span class="mono faint tiny clamp-2 selectable">
                    {[
                      incident.failedActivityId ? `кубик: ${incident.failedActivityId}` : null,
                      processLabel(incident),
                      incident.processInstanceId ? `заявка: ${incident.processInstanceId}` : null,
                      incident.tenantId ? `тенант: ${incident.tenantId}` : null,
                    ]
                      .filter(Boolean)
                      .join(" · ") || "Технические связи отсутствуют"}
                  </span>
                  {#if incident.annotation?.trim()}<span class="muted small truncate">Комментарий: {incident.annotation}</span>{/if}
                </div>
                <Button disabled={!incident.processInstanceId} onclick={() => incident.processInstanceId && app.openProcessInstance(incident.processInstanceId)}>Открыть заявку</Button>
              </div>
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
    gap: 12px;
    min-height: 0;
    height: 100%;
  }
  .filters {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .filters-top {
    gap: 10px;
    align-items: flex-start;
  }
  .filters :global(.bar) {
    box-shadow: none;
    border: 0;
    padding: 0;
    background: transparent;
  }
  .version {
    width: 300px;
    flex: none;
  }
  .bold {
    font-weight: 700;
    white-space: nowrap;
  }
  .types {
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--ink-2);
    border-radius: 999px;
    padding: 5px 11px;
    font-size: 10.5px;
    font-weight: 600;
    transition: border-color var(--dur-fast), color var(--dur-fast), background-color var(--dur-fast), transform var(--dur-fast) var(--ease-spring);
  }
  .chip:hover {
    border-color: var(--border-strong);
    color: var(--ink);
  }
  .chip.active {
    border-color: var(--accent);
    color: var(--accent-ink);
    background: var(--accent-soft);
  }
  .chip:active {
    transform: scale(0.96);
  }
  .gap {
    gap: 8px;
  }
  .skeleton {
    height: 100px;
  }
  .list {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .select-all {
    padding: 0 4px;
    flex: none;
  }
  .pick-all {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    cursor: pointer;
  }
  .pick-all input {
    accent-color: var(--accent);
  }
  .pick {
    display: grid;
    place-items: center;
    width: 24px;
    flex: none;
    cursor: pointer;
  }
  .pick input {
    width: 15px;
    height: 15px;
    accent-color: var(--accent);
    cursor: pointer;
  }
  .incident.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .groups {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-right: 4px;
  }
  .groups > :global(*) {
    flex-shrink: 0;
  }
  .group.open {
    border-color: var(--border-strong);
  }
  .group-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    border: 0;
    background: transparent;
    text-align: left;
  }
  .count {
    min-width: 32px;
    color: var(--danger);
    font-weight: 600;
    font-size: 15px;
    flex: none;
    text-align: right;
  }
  .group-title {
    font-size: 12.5px;
    font-weight: 600;
  }
  .group-message {
    font-size: 11.5px;
    color: var(--ink-2);
  }
  .group-body {
    padding: 0 12px 10px 56px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-top: 1px solid var(--border);
    padding-top: 8px;
  }
  .group-incident {
    gap: 10px;
    padding: 2px 0;
  }
  .incident-slot {
    padding-bottom: 8px;
    height: 100%;
  }
  .incident {
    height: 100%;
    padding: 12px 13px;
    gap: 12px;
    border-color: color-mix(in srgb, var(--danger) 35%, transparent);
    transition: border-color var(--dur-fast), box-shadow var(--dur-base);
  }
  .incident:hover {
    border-color: color-mix(in srgb, var(--danger) 65%, transparent);
    box-shadow: var(--shadow-2);
  }
  .gap5 {
    gap: 5px;
  }
  .small {
    font-size: 10.5px;
  }
  .tiny {
    font-size: 9.5px;
  }
  .message-text {
    font-weight: 600;
    font-size: 12.5px;
    line-height: 1.4;
  }
</style>
