<script lang="ts">
  import { flip } from "svelte/animate";
  import Button from "../components/Button.svelte";
  import DurationChart from "../components/DurationChart.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import Icon from "../components/Icon.svelte";
  import SegmentedTabs from "../components/SegmentedTabs.svelte";
  import Sparkline from "../components/Sparkline.svelte";
  import TextField from "../components/TextField.svelte";
  import TimelineChart from "../components/TimelineChart.svelte";
  import AnimatedNumber from "../components/AnimatedNumber.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { app } from "../state/app.svelte";
  import { dashboardStore, DASHBOARD_POLLING_SECONDS } from "../state/dashboard.svelte";
  import { palette } from "../state/palette.svelte";
  import { recents } from "../state/recents.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { api, errorMessage } from "../api";
  import { contains, DATE_PRESET_LABEL, formatDuration, incidentTypeLabel, isLive, shortDefinitionLabel } from "../format";
  import { groupIncidents, relativeTime, retryableCount, type IncidentGroup } from "../incidents";
  import { expand, flipOptions, rise } from "../motion";
  import { LIVE_FILTER, type DashboardDateFilter, type DashboardDatePreset, type ProcessDefinitionSummary } from "../types";

  let appliedFilter = $state<DashboardDateFilter>(LIVE_FILTER);
  let customFrom = $state("");
  let customTo = $state("");
  let attentionTab = $state<"errors" | "waiting">("errors");
  let processQuery = $state("");
  let expandedGroup = $state<string | null>(null);
  let retryGroup = $state<IncidentGroup | null>(null);
  let retryBusy = $state(false);
  let now = $state(Date.now());

  const connection = $derived(app.selectedConnection);
  const entry = $derived(connection ? dashboardStore.entry(connection, appliedFilter) : null);
  const dashboard = $derived(entry?.dashboard ?? null);
  const loading = $derived(entry?.isRefreshing ?? false);
  const live = $derived(isLive(appliedFilter));
  const production = $derived(connection?.environment === "Production");

  $effect(() => {
    const current = connection;
    const filter = appliedFilter;
    palette.refreshSerial;
    if (!current) return;
    let cancelled = false;
    let seconds = DASHBOARD_POLLING_SECONDS;
    void dashboardStore.refresh(current, filter, true);
    dashboardStore.secondsUntilRefresh = seconds;
    const timer = setInterval(() => {
      if (cancelled) return;
      now = Date.now();
      seconds -= 1;
      if (seconds <= 0) {
        void dashboardStore.refresh(current, filter);
        seconds = DASHBOARD_POLLING_SECONDS;
      }
      dashboardStore.secondsUntilRefresh = seconds;
    }, 1000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  });

  function refreshNow() {
    if (connection) void dashboardStore.refresh(connection, appliedFilter, true);
  }

  function selectPreset(preset: DashboardDatePreset) {
    if (preset === "Custom") appliedFilter = { preset, fromDate: customFrom, toDate: customTo };
    else appliedFilter = { preset, fromDate: "", toDate: "" };
  }

  const presets = (["Live", "Today", "Yesterday", "Custom"] as DashboardDatePreset[]).map((id) => ({ id, label: DATE_PRESET_LABEL[id] }));

  const instances = $derived(dashboard ? dashboard.definitions.reduce((sum, d) => sum + d.instances, 0) : 0);
  const incidentsTotal = $derived(dashboard ? dashboard.definitions.reduce((sum, d) => sum + d.incidents, 0) : 0);
  const stuckInstances = $derived(
    dashboard ? dashboard.longestWaitingActivities.filter((item) => item.maximumWaitingMillis >= 4 * 3_600_000).reduce((sum, item) => sum + item.instanceCount, 0) : 0,
  );
  const startedLastHour = $derived(dashboard?.timeline.at(-1)?.started ?? 0);
  const startedTotal = $derived(dashboard ? dashboard.timeline.reduce((sum, point) => sum + point.started, 0) : 0);
  const errorRate = $derived(instances > 0 ? Math.round((incidentsTotal * 1000) / instances) / 10 : 0);
  const startedSeries = $derived(dashboard?.timeline.map((point) => point.started) ?? []);
  const incidentSeries = $derived(dashboard?.timeline.map((point) => point.incidents) ?? []);
  const completedSeries = $derived(dashboard?.timeline.map((point) => point.completed) ?? []);

  const groups = $derived(dashboard ? groupIncidents(dashboard.incidents) : []);
  const activityNames = $derived.by(() => {
    const map = new Map<string, string>();
    for (const item of dashboard?.incidentActivities ?? []) if (item.activityName) map.set(`${item.processDefinitionId}|${item.activityId}`, item.activityName);
    for (const item of dashboard?.longestWaitingActivities ?? []) if (item.activityName) map.set(`${item.processDefinitionId}|${item.activityId}`, item.activityName);
    return map;
  });

  function activityLabel(processDefinitionId: string | null, activityId: string | null): string {
    if (!activityId) return "Без кубика";
    return activityNames.get(`${processDefinitionId}|${activityId}`) ?? activityId;
  }

  const definitionNames = $derived(new Map((dashboard?.definitions ?? []).map((d) => [d.id, d.name?.trim() || d.key])));

  function processLabel(definitionId: string | null): string {
    if (!definitionId) return "—";
    return definitionNames.get(definitionId) ?? shortDefinitionLabel(definitionId);
  }

  const processes = $derived.by(() => {
    if (!dashboard) return [];
    const needle = processQuery.trim();
    return [...dashboard.definitions]
      .filter((d) => !needle || contains(d.name, needle) || contains(d.key, needle) || contains(d.tenantId, needle))
      .sort((a, b) => b.incidents - a.incidents || b.instances - a.instances || displayName(a).localeCompare(displayName(b)));
  });
  const maxProcessInstances = $derived(Math.max(1, ...processes.map((d) => d.instances)));

  function displayName(definition: ProcessDefinitionSummary): string {
    return definition.name?.trim() ? definition.name : definition.key;
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

  function waitingTone(millis: number): string {
    if (millis >= 24 * 3_600_000) return "danger";
    if (millis >= 4 * 3_600_000) return "warning";
    return "neutral";
  }

  const recentItems = $derived([...recents.pinned, ...recents.recent].filter((item) => item.connectionId === connection?.id).slice(0, 8));
</script>

<div class="page scroll">
  <div class="head row">
    <div class="col grow">
      <h1 class="title">Монитор</h1>
      <span class="muted subtitle">{connection ? `${connection.name} · ${live ? "текущее состояние движка" : dashboard?.timelineTitle ?? "выбранный период"}` : "Подключение не выбрано"}</span>
    </div>
    {#if connection}
      <SegmentedTabs items={presets} selected={appliedFilter.preset} onselect={selectPreset} size="sm" tone="primary" />
      <span class="faint countdown">{loading ? "обновляем…" : `через ${dashboardStore.secondsUntilRefresh} с`}</span>
      <Button size="sm" loading={loading} onclick={refreshNow}><Icon name="refresh" size={13} /> Обновить</Button>
    {/if}
  </div>

  {#if connection && appliedFilter.preset === "Custom"}
    <div class="row custom" transition:expand={{ duration: 200 }}>
      <TextField size="sm" label="С" placeholder="ГГГГ-ММ-ДД" bind:value={customFrom} mono class="date" onenter={() => selectPreset("Custom")} />
      <TextField size="sm" label="По" placeholder="ГГГГ-ММ-ДД" bind:value={customTo} mono class="date" onenter={() => selectPreset("Custom")} />
      <Button size="sm" variant="primary" onclick={() => selectPreset("Custom")}>Применить</Button>
    </div>
  {/if}

  {#if !connection}
    <EmptyPanel title="Добавьте первое окружение Camunda" description="Укажите адрес REST API, чтобы увидеть, что требует внимания." actionLabel="Добавить подключение" onaction={() => app.openConnectionDialog()} />
  {:else}
    {#if entry?.error}
      <div transition:expand><div class="message message-danger selectable">{entry.error}</div></div>
    {/if}

    {#if !dashboard}
      {#if !entry?.error}
        <div class="kpis">{#each [0, 1, 2, 3, 4, 5] as index (index)}<div class="card shimmer kpi-skeleton"></div>{/each}</div>
        <div class="grid"><div class="card shimmer block-skeleton"></div><div class="card shimmer block-skeleton"></div></div>
      {/if}
    {:else}
      <div class="kpis" in:rise>
        <button type="button" class="kpi card" class:alert={incidentsTotal > 0} onclick={() => app.openIncidents(null, appliedFilter)}>
          <span class="kpi-label">{live ? "Открытые инциденты" : "Создано инцидентов"}</span>
          <span class="kpi-value" class:danger={incidentsTotal > 0}><AnimatedNumber value={incidentsTotal} /></span>
          <Sparkline values={incidentSeries} tone="danger" />
        </button>
        <div class="kpi card">
          <span class="kpi-label">{live ? "Заявок в работе" : "Стартовало"}</span>
          <span class="kpi-value"><AnimatedNumber value={instances} /></span>
          <Sparkline values={startedSeries} tone="accent" />
        </div>
        <button type="button" class="kpi card" class:warn={stuckInstances > 0} onclick={() => (attentionTab = "waiting")}>
          <span class="kpi-label">Висят дольше 4 часов</span>
          <span class="kpi-value" class:warning={stuckInstances > 0}><AnimatedNumber value={stuckInstances} /></span>
          <span class="kpi-caption">{dashboard.longestWaitingActivities.length} кубиков с ожиданием</span>
        </button>
        <div class="kpi card">
          <span class="kpi-label">{live ? "Стартов за час" : "Стартов всего"}</span>
          <span class="kpi-value"><AnimatedNumber value={live ? startedLastHour : startedTotal} /></span>
          <span class="kpi-caption">{live ? `${startedTotal} за 24 ч` : dashboard.timeline.length ? `${dashboard.timeline.length} точек на графике` : "нет данных"}</span>
        </div>
        <div class="kpi card">
          <span class="kpi-label">Завершено</span>
          <span class="kpi-value healthy"><AnimatedNumber value={dashboard.completedInstances} /></span>
          <Sparkline values={completedSeries} tone="healthy" />
        </div>
        <div class="kpi card">
          <span class="kpi-label">Доля с ошибками</span>
          <span class="kpi-value" class:danger={errorRate >= 5} class:warning={errorRate > 0 && errorRate < 5}>{errorRate}%</span>
          <span class="kpi-caption">{formatDuration(dashboard.medianDurationMillis)} медиана · P95 {formatDuration(dashboard.p95DurationMillis)}</span>
        </div>
      </div>

      <div class="grid" in:rise={{ delay: 40 }}>
        <section class="card attention">
          <div class="card-head">
            <Icon name="inbox" size={15} class="head-icon" />
            <span class="card-title">Требует внимания</span>
            <span class="grow"></span>
            <SegmentedTabs
              items={[{ id: "errors", label: "Группы ошибок", badge: groups.length }, { id: "waiting", label: "Долгое ожидание", badge: dashboard.longestWaitingActivities.length }]}
              selected={attentionTab}
              onselect={(id) => (attentionTab = id)}
              size="sm"
            />
          </div>
          {#if attentionTab === "errors"}
            {#if groups.length === 0}
              <div class="calm"><Icon name="check" size={18} /><span>Инцидентов нет — все процессы идут штатно</span></div>
            {:else}
              <div class="groups">
                {#each groups.slice(0, 12) as group (group.key)}
                  {@const open = expandedGroup === group.key}
                  <div class="group" class:open animate:flip={flipOptions}>
                    <button type="button" class="group-row" onclick={() => (expandedGroup = open ? null : group.key)}>
                      <span class="count mono">{group.count}</span>
                      <span class="col grow">
                        <span class="row">
                          <span class="group-title truncate">{activityLabel(group.processDefinitionId, group.activityId)}</span>
                          <span class="pill pill-danger">{incidentTypeLabel(group.type)}</span>
                        </span>
                        <span class="group-message truncate selectable">{group.signature}</span>
                        <span class="group-meta faint truncate">{processLabel(group.processDefinitionId)} · заявок: {group.instanceIds.length} · последний {relativeTime(group.latestTimestamp, now)}</span>
                      </span>
                      <Icon name="chevron" size={14} class="chevron {open ? 'flipped' : ''}" />
                    </button>
                    {#if open}
                      <div class="group-body" transition:expand={{ duration: 200 }}>
                        <div class="row group-actions">
                          <Button size="sm" variant="primary" disabled={retryableCount(group) === 0} onclick={() => (retryGroup = group)}>
                            <Icon name="play" size={12} /> Повторить все ({retryableCount(group)})
                          </Button>
                          <Button size="sm" onclick={() => app.openIncidents(group.processDefinitionId, appliedFilter)}>Открыть инциденты</Button>
                          {#if group.processDefinitionId}
                            <Button size="sm" variant="ghost" onclick={() => app.openProcessDefinition(group.processDefinitionId!, appliedFilter)}>Процесс</Button>
                          {/if}
                        </div>
                        {#if group.sampleMessage}
                          <pre class="sample selectable">{group.sampleMessage.slice(0, 600)}</pre>
                        {/if}
                        <div class="instances">
                          {#each group.instanceIds.slice(0, 12) as instanceId (instanceId)}
                            <button type="button" class="instance-chip mono" onclick={() => app.openProcessInstance(instanceId)}>{instanceId.slice(0, 8)}…</button>
                          {/each}
                          {#if group.instanceIds.length > 12}<span class="faint">и ещё {group.instanceIds.length - 12}</span>{/if}
                        </div>
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          {:else if dashboard.longestWaitingActivities.length === 0}
            <div class="calm"><Icon name="check" size={18} /><span>Незавершённых кубиков с доступной историей нет</span></div>
          {:else}
            <div class="waiting">
              {#each dashboard.longestWaitingActivities as item (item.processDefinitionId + item.activityId)}
                <button type="button" class="wait-row tone-{waitingTone(item.maximumWaitingMillis)}" onclick={() => app.openProcessInstance(item.oldestProcessInstanceId)}>
                  <span class="col grow">
                    <span class="group-title truncate">{item.activityName ?? item.topic ?? item.activityId}</span>
                    <span class="group-meta faint truncate">{processLabel(item.processDefinitionId)} · {item.activityType}{item.topic ? ` · topic: ${item.topic}` : ""}</span>
                  </span>
                  <span class="col stat"><span class="eyebrow">заявок</span><span class="mono">{item.instanceCount}</span></span>
                  <span class="col stat"><span class="eyebrow">среднее</span><span class="mono">{formatDuration(item.averageWaitingMillis)}</span></span>
                  <span class="col stat"><span class="eyebrow">максимум</span><span class="mono wait-max">{formatDuration(item.maximumWaitingMillis)}</span></span>
                  <Icon name="chevronRight" size={14} class="chevron" />
                </button>
              {/each}
            </div>
          {/if}
        </section>

        <div class="side">
          <section class="card processes">
            <div class="card-head">
              <Icon name="layers" size={15} class="head-icon" />
              <span class="card-title">Процессы</span>
              <span class="faint">{dashboard.definitions.length}</span>
              <span class="grow"></span>
              <TextField size="sm" placeholder="Фильтр" bind:value={processQuery} type="search" class="process-filter" />
            </div>
            <div class="process-list scroll">
              {#each processes as definition (definition.id)}
                <div class="process" role="button" tabindex="0" animate:flip={flipOptions} onclick={() => app.openProcessDefinition(definition.id, appliedFilter)} onkeydown={(event) => event.key === "Enter" && app.openProcessDefinition(definition.id, appliedFilter)}>
                  <span class="col grow">
                    <span class="row">
                      <span class="process-name truncate">{displayName(definition)}</span>
                      {#if definition.suspended}<span class="pill pill-warning">пауза</span>{/if}
                    </span>
                    <span class="bar"><span class="bar-fill" style:width="{(definition.instances / maxProcessInstances) * 100}%"></span></span>
                  </span>
                  <span class="col stat"><span class="eyebrow">{live ? "в работе" : "стартов"}</span><span class="mono">{definition.instances}</span></span>
                  <button type="button" class="col stat incident-stat" class:danger={definition.incidents > 0} disabled={definition.incidents === 0} onclick={(event) => { event.stopPropagation(); app.openIncidents(definition.id, appliedFilter); }}>
                    <span class="eyebrow">инц.</span><span class="mono">{definition.incidents}</span>
                  </button>
                </div>
              {:else}
                <div class="calm small">Процессы не найдены</div>
              {/each}
            </div>
          </section>

          <section class="card recents">
            <div class="card-head">
              <Icon name="history" size={15} class="head-icon" />
              <span class="card-title">Недавние заявки</span>
              <span class="grow"></span>
              <button type="button" class="link small" onclick={() => palette.show()}>Найти ⌘K</button>
            </div>
            <div class="recent-list">
              {#each recentItems as item (item.processInstanceId)}
                <button type="button" class="recent" onclick={() => app.openProcessInstance(item.processInstanceId, item.connectionId)}>
                  <Icon name={item.pinned ? "star" : "file"} size={14} class={item.pinned ? "star" : ""} />
                  <span class="col grow">
                    <span class="truncate" class:mono={!item.businessKey}>{item.businessKey ?? item.processInstanceId}</span>
                    <span class="faint truncate small">{item.processDefinitionKey ?? "—"} · {relativeTime(new Date(item.openedAt).toISOString(), now)}</span>
                  </span>
                </button>
              {:else}
                <div class="calm small">Открытые заявки появятся здесь</div>
              {/each}
            </div>
          </section>
        </div>
      </div>

      <div class="charts" in:rise={{ delay: 80 }}>
        <div class="chart-main"><TimelineChart points={dashboard.timeline} title={dashboard.timelineTitle} expanded={false} ontoggle={() => undefined} /></div>
        <DurationChart buckets={dashboard.durationBuckets} average={dashboard.averageDurationMillis} median={dashboard.medianDurationMillis} p95={dashboard.p95DurationMillis} />
      </div>
      {#if dashboard.timelineTruncated}
        <p class="warning tiny">На графике показаны первые 20 000 записей каждого типа. Итоговые счётчики остаются точными.</p>
      {/if}
    {/if}
  {/if}
</div>

<ConfirmDialog
  open={retryGroup != null}
  description={retryGroup ? `Выставить 3 попытки для ${retryableCount(retryGroup)} заданий и внешних задач в группе «${retryGroup.signature.slice(0, 80)}»? Camunda сразу начнёт повторное выполнение.` : ""}
  production={production}
  busy={retryBusy}
  onconfirm={confirmRetry}
  oncancel={() => (retryGroup = null)}
/>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    height: 100%;
    padding-right: 4px;
    padding-bottom: 20px;
  }
  .page > :global(*) {
    flex-shrink: 0;
  }
  .head {
    gap: 10px;
  }
  .title {
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  .subtitle {
    font-size: 12px;
  }
  .countdown {
    font-size: 11px;
    white-space: nowrap;
  }
  .custom {
    gap: 8px;
    align-items: flex-end;
  }
  .custom :global(.date) {
    width: 150px;
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    overflow: hidden;
  }
  .kpi {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 14px;
    min-width: 0;
    text-align: left;
    border: 0;
    border-right: 1px solid var(--border);
    border-radius: 0;
    background: transparent;
    transition: background-color var(--dur-fast);
  }
  .kpi:last-child {
    border-right: 0;
  }
  button.kpi:hover {
    background: var(--surface-2);
  }
  .kpi-skeleton {
    height: 86px;
  }
  .kpi-label {
    font-size: 10.5px;
    font-weight: 500;
    color: var(--ink-2);
  }
  .kpi-value {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
  }
  .kpi-caption {
    font-size: 10px;
    color: var(--ink-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .danger { color: var(--danger); }
  .warning { color: var(--warning); }
  .healthy { color: var(--healthy); }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1.55fr) minmax(320px, 1fr);
    gap: 12px;
    align-items: start;
  }
  .block-skeleton {
    height: 360px;
  }
  :global(.head-icon) {
    color: var(--ink-3);
  }
  .attention {
    display: flex;
    flex-direction: column;
    min-height: 360px;
  }
  .calm {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    flex: 1;
    min-height: 120px;
    color: var(--healthy);
    font-size: 12.5px;
  }
  .calm.small {
    min-height: 60px;
    color: var(--ink-3);
    font-size: 11.5px;
  }
  .groups {
    display: flex;
    flex-direction: column;
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
    gap: 12px;
    width: 100%;
    padding: 10px 14px;
    border: 0;
    background: transparent;
    text-align: left;
    transition: background-color var(--dur-fast);
  }
  .group-row:hover {
    background: var(--surface-2);
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
  .group-meta {
    font-size: 10.5px;
  }
  .group-body {
    padding: 0 14px 12px 58px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .group-actions {
    gap: 6px;
  }
  .sample {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-xs);
    background: var(--surface);
    border: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 10.5px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 120px;
    overflow: auto;
    color: var(--ink-2);
  }
  .instances {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    align-items: center;
    font-size: 10.5px;
  }
  .instance-chip {
    padding: 2px 0;
    border: 0;
    background: transparent;
    font-size: 11px;
    color: var(--accent);
  }
  .instance-chip:hover {
    text-decoration: underline;
  }
  .waiting {
    display: flex;
    flex-direction: column;
  }
  .wait-row {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    padding: 10px 14px;
    border: 0;
    border-bottom: 1px solid var(--border);
    border-left: 3px solid transparent;
    background: transparent;
    text-align: left;
    transition: background-color var(--dur-fast);
  }
  .wait-row:last-child {
    border-bottom: 0;
  }
  .wait-row:hover {
    background: var(--surface-2);
  }
  .wait-row.tone-danger { border-left-color: var(--danger); }
  .wait-row.tone-danger .wait-max { color: var(--danger); }
  .wait-row.tone-warning { border-left-color: var(--warning); }
  .wait-row.tone-warning .wait-max { color: var(--warning); }
  .stat {
    min-width: 64px;
    align-items: flex-end;
    text-align: right;
    font-size: 12px;
    flex: none;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .processes :global(.process-filter) {
    width: 150px;
  }
  .process-list {
    max-height: 320px;
    display: flex;
    flex-direction: column;
  }
  .process {
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 9px 14px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    text-align: left;
    transition: background-color var(--dur-fast);
  }
  .process:last-child {
    border-bottom: 0;
  }
  .process:hover {
    background: var(--surface-2);
  }
  .process-name {
    font-size: 12.5px;
    font-weight: 500;
  }
  .bar {
    display: block;
    height: 3px;
    margin-top: 5px;
    border-radius: 2px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width var(--dur-slow) var(--ease-out);
  }
  .incident-stat {
    border: 0;
    background: transparent;
    padding: 0;
    cursor: default;
  }
  .incident-stat.danger {
    cursor: pointer;
  }
  .incident-stat.danger:hover {
    text-decoration: underline;
  }
  .recent-list {
    display: flex;
    flex-direction: column;
  }
  .recent {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 14px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    text-align: left;
    font-size: 12px;
    color: var(--ink-2);
    transition: background-color var(--dur-fast);
  }
  .recent:last-child {
    border-bottom: 0;
  }
  .recent:hover {
    background: var(--surface-2);
  }
  :global(.star) {
    color: var(--warning);
  }
  .small {
    font-size: 10.5px;
  }
  .charts {
    display: grid;
    grid-template-columns: minmax(0, 1.55fr) minmax(320px, 1fr);
    gap: 12px;
    height: 240px;
  }
  .chart-main {
    display: grid;
    min-height: 0;
  }
  .chart-main :global(.chart) {
    height: 100%;
  }
  .charts :global(.chart) {
    height: 100%;
  }
  .tiny {
    font-size: 10px;
  }
</style>
