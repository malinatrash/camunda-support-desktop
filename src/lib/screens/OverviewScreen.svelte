<script lang="ts">
  import Button from "../components/Button.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import Icon from "../components/Icon.svelte";
  import PeriodPicker from "../components/PeriodPicker.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { app } from "../state/app.svelte";
  import { dashboardStore, DASHBOARD_POLLING_SECONDS } from "../state/dashboard.svelte";
  import { palette } from "../state/palette.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { api, errorMessage } from "../api";
  import { contains, definitionKeyOf, formatDuration, grouped, incidentTypeLabel, isLive } from "../format";
  import { groupIncidents, relativeTime, retryableCount, type IncidentGroup } from "../incidents";
  import { expand } from "../motion";
  import { LIVE_FILTER, type DashboardDateFilter, type ProcessDefinitionSummary } from "../types";

  const STUCK_MILLIS = 4 * 3_600_000;

  let filter = $state<DashboardDateFilter>(app.selectedDashboardDateFilter ?? LIVE_FILTER);
  let tab = $state<"errors" | "waiting">("errors");
  let processQuery = $state("");
  let expandedGroup = $state<string | null>(null);
  let retryGroup = $state<IncidentGroup | null>(null);
  let retryBusy = $state(false);
  let now = $state(Date.now());

  const connection = $derived(app.selectedConnection);
  const entry = $derived(connection ? dashboardStore.entry(connection, filter) : null);
  const dashboard = $derived(entry?.dashboard ?? null);
  const loading = $derived(entry?.isRefreshing ?? false);
  const live = $derived(isLive(filter));

  $effect(() => {
    const current = connection;
    const period = filter;
    palette.refreshSerial;
    if (!current) return;
    void dashboardStore.refresh(current, period, true);
    const timer = setInterval(() => {
      now = Date.now();
      void dashboardStore.refresh(current, period);
    }, 1000);
    return () => clearInterval(timer);
  });

  function refreshNow() {
    if (connection) void dashboardStore.refresh(connection, filter, true);
  }

  const status = $derived.by(() => {
    if (loading && !dashboard) return "загружаем…";
    if (!entry?.lastSuccessfulRefresh) return "";
    const seconds = Math.max(0, Math.round((now - entry.lastSuccessfulRefresh) / 1000));
    return seconds < 2 ? "обновлено только что" : `обновлено ${seconds} с назад`;
  });

  const instances = $derived(dashboard ? dashboard.definitions.reduce((sum, d) => sum + d.instances, 0) : 0);
  const incidentsTotal = $derived(dashboard ? dashboard.definitions.reduce((sum, d) => sum + d.incidents, 0) : 0);
  const stuck = $derived(
    dashboard
      ? dashboard.longestWaitingActivities.filter((item) => item.maximumWaitingMillis >= STUCK_MILLIS).reduce((sum, item) => sum + item.instanceCount, 0)
      : 0,
  );
  const groups = $derived(dashboard ? groupIncidents(dashboard.incidents) : []);

  const namesByKey = $derived(new Map((dashboard?.definitions ?? []).map((d) => [d.key, displayName(d)])));
  const activityNames = $derived.by(() => {
    const map = new Map<string, string>();
    for (const item of dashboard?.incidentActivities ?? []) if (item.activityName) map.set(`${item.processDefinitionId}|${item.activityId}`, item.activityName);
    for (const item of dashboard?.longestWaitingActivities ?? []) if (item.activityName) map.set(`${item.processDefinitionId}|${item.activityId}`, item.activityName);
    return map;
  });

  const processes = $derived.by(() => {
    if (!dashboard) return [];
    const needle = processQuery.trim();
    return dashboard.definitions
      .filter((d) => !needle || contains(d.name, needle) || contains(d.key, needle) || contains(d.tenantId, needle))
      .sort((a, b) => b.incidents - a.incidents || b.instances - a.instances || displayName(a).localeCompare(displayName(b)) || b.version - a.version);
  });

  function displayName(definition: ProcessDefinitionSummary): string {
    return definition.name?.trim() || definition.key;
  }

  function processLabel(definitionId: string | null): string {
    const key = definitionId ? definitionKeyOf(definitionId) : null;
    return key ? (namesByKey.get(key) ?? key) : "—";
  }

  function activityLabel(definitionId: string | null, activityId: string | null): string {
    if (!activityId) return "Без кубика";
    return activityNames.get(`${definitionId}|${activityId}`) ?? activityId;
  }

  function selectFilter(next: DashboardDateFilter) {
    filter = next;
    app.selectedDashboardDateFilter = next;
  }

  async function confirmRetry() {
    const group = retryGroup;
    if (!group || !connection) return;
    retryBusy = true;
    try {
      const result = await api.retryIncidents(connection, group.incidents, 3);
      toasts.success(`Повтор запущен: заданий ${result.jobs}, внешних задач ${result.externalTasks}${result.skipped ? `, пропущено ${result.skipped}` : ""}.`);
      retryGroup = null;
      refreshNow();
    } catch (error) {
      toasts.error(errorMessage(error));
    } finally {
      retryBusy = false;
    }
  }
</script>

<div class="page scroll">
  <header class="head">
    {#if connection}
      <PeriodPicker value={filter} onchange={selectFilter} />
      <span class="grow"></span>
      <span class="faint status">{status}</span>
      <button type="button" class="icon-button" class:spinning={loading} title="Обновить · каждые {DASHBOARD_POLLING_SECONDS} с" onclick={refreshNow}>
        <Icon name="refresh" size={15} />
      </button>
    {/if}
  </header>

  {#if !connection}
    <EmptyPanel title="Добавьте подключение" description="Укажите адрес REST API Camunda, чтобы увидеть, что требует внимания." actionLabel="Добавить подключение" onaction={() => app.openConnectionDialog()} />
  {:else}
    {#if entry?.error}
      <div transition:expand><div class="message message-danger selectable">{entry.error}</div></div>
    {/if}

    {#if !dashboard}
      {#if !entry?.error}
        <div class="skeleton">{#each [0, 1, 2, 3, 4] as index (index)}<div class="shimmer line"></div>{/each}</div>
      {/if}
    {:else}
      <div class="stats">
        <button type="button" class="stat" class:danger={incidentsTotal > 0} onclick={() => app.openIncidents(null, filter)}>
          <span class="value">{grouped(incidentsTotal)}</span><span class="label">{live ? "инцидентов" : "новых инцидентов"}</span>
        </button>
        <div class="stat"><span class="value">{grouped(instances)}</span><span class="label">{live ? "в работе" : "стартовало"}</span></div>
        <button type="button" class="stat" class:warning={stuck > 0} onclick={() => (tab = "waiting")}>
          <span class="value">{grouped(stuck)}</span><span class="label">висят дольше 4 ч</span>
        </button>
        <div class="stat"><span class="value">{grouped(dashboard.completedInstances)}</span><span class="label">{live ? "завершено за сутки" : "завершено"}</span></div>
      </div>

      <div class="columns">
        <section class="section">
          <div class="section-head">
            <button type="button" class="tab" class:active={tab === "errors"} onclick={() => (tab = "errors")}>Ошибки <span class="counter">{groups.length}</span></button>
            <button type="button" class="tab" class:active={tab === "waiting"} onclick={() => (tab = "waiting")}>Ожидание <span class="counter">{dashboard.longestWaitingActivities.length}</span></button>
          </div>

          {#if tab === "errors"}
            {#each groups.slice(0, 15) as group (group.key)}
              {@const open = expandedGroup === group.key}
              <div class="item" class:open>
                <button type="button" class="item-row" onclick={() => (expandedGroup = open ? null : group.key)}>
                  <span class="count danger">{group.count}</span>
                  <span class="col grow">
                    <span class="item-title truncate">{activityLabel(group.processDefinitionId, group.activityId)}</span>
                    <span class="item-text truncate selectable">{group.signature}</span>
                    <span class="faint truncate meta">{processLabel(group.processDefinitionId)} · {incidentTypeLabel(group.type)} · {relativeTime(group.latestTimestamp, now)}</span>
                  </span>
                </button>
                {#if open}
                  <div class="item-body" transition:expand={{ duration: 180 }}>
                    {#if group.sampleMessage}<pre class="sample selectable">{group.sampleMessage.slice(0, 600)}</pre>{/if}
                    <div class="row wrap">
                      <Button size="sm" variant="primary" disabled={retryableCount(group) === 0} onclick={() => (retryGroup = group)}>Повторить ({retryableCount(group)})</Button>
                      <Button size="sm" variant="ghost" onclick={() => app.openIncidents(group.processDefinitionId, filter)}>Все инциденты</Button>
                      {#each group.instanceIds.slice(0, 6) as instanceId (instanceId)}
                        <button type="button" class="link mono small" onclick={() => app.openProcessInstance(instanceId)}>{instanceId.slice(0, 8)}</button>
                      {/each}
                      {#if group.instanceIds.length > 6}<span class="faint small">+{group.instanceIds.length - 6}</span>{/if}
                    </div>
                  </div>
                {/if}
              </div>
            {:else}
              <p class="calm">Инцидентов нет</p>
            {/each}
          {:else}
            {#each dashboard.longestWaitingActivities as item (item.processDefinitionId + item.activityId)}
              <button type="button" class="item-row" onclick={() => app.openProcessInstance(item.oldestProcessInstanceId)}>
                <span class="count" class:danger={item.maximumWaitingMillis >= 24 * 3_600_000} class:warning={item.maximumWaitingMillis >= STUCK_MILLIS && item.maximumWaitingMillis < 24 * 3_600_000}>{formatDuration(item.maximumWaitingMillis)}</span>
                <span class="col grow">
                  <span class="item-title truncate">{item.activityName ?? item.topic ?? item.activityId}</span>
                  <span class="faint truncate meta">{processLabel(item.processDefinitionId)} · заявок {item.instanceCount} · в среднем {formatDuration(item.averageWaitingMillis)}</span>
                </span>
              </button>
            {:else}
              <p class="calm">Долго ожидающих шагов нет</p>
            {/each}
          {/if}
        </section>

        <section class="section">
          <div class="section-head">
            <span class="section-title">Процессы <span class="counter">{dashboard.definitions.length}</span></span>
            <span class="grow"></span>
            <input class="filter" type="search" placeholder="Фильтр" bind:value={processQuery} />
          </div>
          <div class="process-head faint">
            <span class="grow">Процесс</span><span class="num">{live ? "в работе" : "стартов"}</span><span class="num">инциденты</span>
          </div>
          {#each processes as definition (definition.id)}
            <button type="button" class="process" onclick={() => app.openProcessDefinition(definition.id, filter)}>
              <span class="col grow">
                <span class="truncate">{displayName(definition)}</span>
                <span class="faint mono meta truncate">{definition.key} · v{definition.version}{definition.suspended ? " · пауза" : ""}</span>
              </span>
              <span class="num mono">{grouped(definition.instances)}</span>
              <span class="num mono">{#if definition.incidents > 0}<span class="pill pill-danger">{definition.incidents}</span>{:else}<span class="faint">0</span>{/if}</span>
            </button>
          {:else}
            <p class="calm">Процессы не найдены</p>
          {/each}
        </section>
      </div>
    {/if}
  {/if}
</div>

<ConfirmDialog
  open={retryGroup != null}
  description={retryGroup ? `Выставить 3 попытки для ${retryableCount(retryGroup)} заданий и внешних задач в группе «${retryGroup.signature.slice(0, 80)}»? Camunda сразу начнёт повторное выполнение.` : ""}
  production={connection?.environment === "Production"}
  busy={retryBusy}
  onconfirm={confirmRetry}
  oncancel={() => (retryGroup = null)}
/>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 20px;
    height: 100%;
    padding-bottom: 24px;
  }
  .page > :global(*) {
    flex-shrink: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 30px;
  }
  .head :global(.picker) {
    margin-left: -9px;
  }
  .status {
    font-size: var(--text-sm);
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
  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .line {
    height: 18px;
  }
  .line:first-child {
    height: 40px;
    width: 60%;
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 40px;
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-2);
  }
  .stat {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
  }
  button.stat:hover .label {
    color: var(--ink);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .value {
    font-size: var(--text-3xl);
    font-weight: 600;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }
  .label {
    color: var(--ink-2);
  }
  .danger,
  .stat.danger .value {
    color: var(--danger);
  }
  .warning,
  .stat.warning .value {
    color: var(--warning);
  }
  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: 40px;
    align-items: start;
  }
  .section {
    display: flex;
    flex-direction: column;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    overflow: hidden;
  }
  .section > :global(:last-child) {
    border-bottom: 0;
  }
  .section-head {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 40px;
    padding: 0 12px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
  }
  .section-title,
  .tab {
    font-weight: 600;
  }
  .tab {
    height: 100%;
    padding: 0;
    border: 0;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    background: transparent;
    color: var(--ink-2);
  }
  .tab.active {
    color: var(--ink);
    border-bottom-color: var(--accent);
  }
  .filter {
    width: 160px;
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    outline: none;
  }
  .filter:focus {
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  .item {
    border-bottom: 1px solid var(--border);
  }
  .item-row {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    width: 100%;
    padding: 10px 12px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    text-align: left;
  }
  .item-row:hover,
  .process:hover {
    background: var(--row-hover);
  }
  .item .item-row {
    border-bottom: 0;
  }
  .item-row:hover .item-title {
    color: var(--accent);
  }
  .count {
    min-width: 52px;
    text-align: right;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .item-title {
    font-weight: 500;
  }
  .item-text {
    color: var(--ink-2);
    font-size: var(--text-md);
  }
  .meta,
  .small {
    font-size: var(--text-sm);
  }
  .item-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 12px 12px 78px;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .sample {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 140px;
    overflow: auto;
    color: var(--ink-2);
  }
  .calm {
    padding: 24px 12px;
    color: var(--ink-3);
  }
  .process-head {
    display: flex;
    gap: 12px;
    padding: 6px 12px;
    font-size: var(--text-sm);
    font-weight: 600;
    border-bottom: 1px solid var(--border);
  }
  .process {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 12px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    text-align: left;
  }
  .process:hover .truncate:first-child {
    color: var(--accent);
  }
  .num {
    width: 80px;
    flex: none;
    text-align: right;
  }
</style>
