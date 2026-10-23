<script lang="ts">
  import { flip } from "svelte/animate";
  import Button from "../components/Button.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import SegmentedTabs from "../components/SegmentedTabs.svelte";
  import StateBadge from "../components/StateBadge.svelte";
  import TeleportDialog from "../components/TeleportDialog.svelte";
  import TextField from "../components/TextField.svelte";
  import VariableEditDialog from "../components/VariableEditDialog.svelte";
  import BpmnViewer from "../bpmn/BpmnViewer.svelte";
  import Icon from "../components/Icon.svelte";
  import { app } from "../state/app.svelte";
  import { instanceStore, INSTANCE_POLLING_SECONDS } from "../state/instances.svelte";
  import { palette } from "../state/palette.svelte";
  import { recents } from "../state/recents.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { api, copyText, errorMessage } from "../api";
  import { processInstanceCockpitUrl } from "../connection";
  import { contains, isVariableEditable } from "../format";
  import { expand, flipOptions, rise, screenIn } from "../motion";
  import { teleportTargets, type ActiveActivityInstance, type BpmnNode, type CamundaConnection, type CurrentActivityEvidence, type ProcessVariable, type TeleportRequest } from "../types";

  let { connection, processInstanceId }: { connection: CamundaConnection; processInstanceId: string } = $props();

  type Tab = "diagram" | "timeline" | "diagnosis" | "variables" | "metadata";
  type PendingAction =
    | { kind: "jobRetry"; jobId: string }
    | { kind: "externalRetry"; taskId: string }
    | { kind: "externalUnlock"; taskId: string }
    | { kind: "suspension"; suspended: boolean };

  const entry = $derived(instanceStore.entry(connection, processInstanceId));
  const details = $derived(entry.details);
  const loading = $derived(entry.isRefreshing);
  const production = $derived(connection.environment === "Production");

  let operationError = $state<string | null>(null);
  let operationMessage = $state<string | null>(null);
  let operationBusy = $state(false);
  let secondsUntilRefresh = $state(INSTANCE_POLLING_SECONDS);
  let teleportOpen = $state(false);
  let teleportTargetId = $state<string | null>(null);
  let selectedNodeId = $state<string | null>(null);
  let variableToEdit = $state<ProcessVariable | null>(null);
  let variableEditError = $state<string | null>(null);
  let pendingAction = $state<PendingAction | null>(null);
  let tab = $state<Tab>("diagram");
  let variableQuery = $state("");
  const pinned = $derived(recents.isPinned(connection.id, processInstanceId));

  $effect(() => {
    if (palette.refreshSerial > 0) void refreshNow();
  });

  function togglePin() {
    const now = recents.togglePin(connection.id, processInstanceId);
    toasts.success(now ? "Заявка закреплена в недавних." : "Закрепление снято.");
  }

  // Markdown-отчёт о состоянии заявки для обращения или чата.
  function buildReport(): string {
    const snapshot = details;
    if (!snapshot) return "";
    const lines: string[] = [];
    lines.push(`# Заявка ${snapshot.instance.businessKey ?? snapshot.instance.id}`);
    lines.push(`- Подключение: ${connection.name} (${connection.restUrl})`);
    lines.push(`- Процесс: ${snapshot.instance.definitionKey} · ${snapshot.instance.definitionId}`);
    lines.push(`- ID заявки: ${snapshot.instance.id}`);
    lines.push(`- Состояние: ${snapshot.instance.ended ? "завершена" : snapshot.instance.suspended ? "приостановлена" : "запущена"}`);
    lines.push(`- Снимок: ${new Date().toISOString()}`);
    lines.push("", "## Текущие позиции");
    for (const activity of snapshot.activeActivities) {
      lines.push(`- ${activity.activityName ?? activity.activityId} (${activity.activityId}) — ${activity.evidence.join(", ")}`);
    }
    if (snapshot.activeActivities.length === 0) lines.push("- нет");
    lines.push("", `## Инциденты (${snapshot.incidents.length})`);
    for (const incident of snapshot.incidents) {
      lines.push(`- ${incident.type} · ${incident.failedActivityId ?? incident.activityId ?? "—"} · ${incident.timestamp ?? ""}`);
      if (incident.message) lines.push("  ```", `  ${incident.message.split("\n").join("\n  ")}`, "  ```");
    }
    if (snapshot.jobs.length > 0) {
      lines.push("", "## Задания движка");
      for (const job of snapshot.jobs) lines.push(`- ${job.id} · попыток ${job.retries} · ${job.exceptionMessage ?? "без ошибки"}`);
    }
    if (snapshot.externalTasks.length > 0) {
      lines.push("", "## Внешние задачи");
      for (const task of snapshot.externalTasks) lines.push(`- ${task.topicName} · ${task.id} · попыток ${task.retries ?? "—"} · ${task.errorMessage ?? "без ошибки"}`);
    }
    lines.push("", "## Переменные");
    for (const variable of snapshot.variables) lines.push(`- ${variable.name} (${variable.type}): ${variable.value.length > 200 ? `${variable.value.slice(0, 200)}…` : variable.value}`);
    return lines.join("\n");
  }

  async function copyReport() {
    await copy(buildReport(), "Отчёт по заявке скопирован в Markdown.", "Не удалось скопировать отчёт.");
  }

  const timeline = $derived(details ? [...details.activityTimeline].sort((a, b) => (a.startTime ?? "").localeCompare(b.startTime ?? "")) : []);

  function formatTime(value: string | null): string {
    return value ? value.slice(11, 23) : "—";
  }

  function formatShort(millis: number | null): string {
    if (millis == null) return "в работе";
    if (millis < 1000) return `${millis} мс`;
    if (millis < 60_000) return `${(millis / 1000).toFixed(1)} с`;
    if (millis < 3_600_000) return `${Math.round(millis / 60_000)} мин`;
    return `${(millis / 3_600_000).toFixed(1)} ч`;
  }

  $effect(() => {
    let cancelled = false;
    operationError = null;
    let seconds = INSTANCE_POLLING_SECONDS;
    const refresh = () =>
      instanceStore.refresh(connection, processInstanceId).then((loaded) => {
        if (!cancelled && loaded.details) app.updateOpenProcessTab(loaded.details.instance);
      });
    void refresh();
    secondsUntilRefresh = seconds;
    const timer = setInterval(() => {
      if (cancelled) return;
      seconds -= 1;
      if (seconds <= 0) {
        void refresh();
        seconds = INSTANCE_POLLING_SECONDS;
      }
      secondsUntilRefresh = seconds;
    }, 1000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  });

  async function refreshNow() {
    operationError = null;
    const loaded = await instanceStore.refresh(connection, processInstanceId, true);
    if (loaded.details) app.updateOpenProcessTab(loaded.details.instance);
    secondsUntilRefresh = INSTANCE_POLLING_SECONDS;
  }

  async function runOperation(operation: () => Promise<void>, successMessage: string): Promise<boolean> {
    operationBusy = true;
    operationError = null;
    try {
      await operation();
      operationMessage = successMessage;
      toasts.success(successMessage);
      await refreshNow();
      return true;
    } catch (failure) {
      operationError = errorMessage(failure);
      return false;
    } finally {
      operationBusy = false;
    }
  }

  function execute(action: PendingAction) {
    pendingAction = null;
    switch (action.kind) {
      case "jobRetry":
        void runOperation(() => api.setJobRetries(connection, action.jobId, 3), "Для задания движка установлено 3 попытки.");
        break;
      case "externalRetry":
        void runOperation(() => api.setExternalTaskRetries(connection, action.taskId, 3), "Для внешней задачи установлено 3 попытки.");
        break;
      case "externalUnlock":
        void runOperation(() => api.unlockExternalTask(connection, action.taskId), "Внешняя задача разблокирована.");
        break;
      case "suspension":
        void runOperation(
          () => api.setProcessInstanceSuspended(connection, processInstanceId, action.suspended),
          action.suspended ? "Заявка приостановлена." : "Заявка активирована.",
        );
        break;
    }
  }

  function actionDescription(action: PendingAction): string {
    switch (action.kind) {
      case "jobRetry":
        return `Установить 3 попытки для задания движка ${action.jobId}?`;
      case "externalRetry":
        return `Установить 3 попытки для внешней задачи ${action.taskId}?`;
      case "externalUnlock":
        return `Снять блокировку и обработчика с внешней задачи ${action.taskId}? Другой обработчик сможет забрать её сразу.`;
      default:
        return action.suspended ? "Приостановить экземпляр процесса?" : "Активировать экземпляр процесса?";
    }
  }

  async function executeTeleport(request: TeleportRequest) {
    const ok = await runOperation(() => api.teleportProcessInstance(connection, processInstanceId, request), "Телепорт выполнен, состояние заявки обновлено.");
    if (ok) teleportOpen = false;
  }

  async function saveVariable(newValue: string) {
    const variable = variableToEdit;
    if (!variable) return;
    operationBusy = true;
    variableEditError = null;
    try {
      await api.updateProcessVariable(connection, processInstanceId, { name: variable.name, type: variable.type, value: newValue, valueInfo: variable.valueInfo });
      variableToEdit = null;
      operationMessage = `Переменная ${variable.name} обновлена.`;
      toasts.success(operationMessage);
      await refreshNow();
    } catch (failure) {
      variableEditError = errorMessage(failure);
    } finally {
      operationBusy = false;
    }
  }

  async function copy(text: string, success: string, failure: string) {
    if (await copyText(text)) {
      operationMessage = success;
      operationError = null;
      toasts.success(success);
    } else {
      operationError = failure;
    }
  }

  async function copyLink() {
    const link = await api.processInstanceDeepLink(connection, processInstanceId);
    if (!link) {
      operationError = "Не удалось скопировать ссылку на заявку.";
      return;
    }
    await copy(link, "Ссылка на заявку скопирована. При клике она откроется в приложении.", "Не удалось скопировать ссылку на заявку.");
  }

  const cockpitUrl = $derived(processInstanceCockpitUrl(connection, processInstanceId));
  const targets = $derived(details ? teleportTargets(details.diagram) : []);
  const targetIds = $derived(targets.map((node) => node.id));
  const activeIds = $derived(details ? details.activeActivities.map((activity) => activity.activityId) : []);
  const incidentIds = $derived(details ? details.incidents.map((incident) => incident.failedActivityId ?? incident.activityId).filter((id): id is string => !!id) : []);
  const completedCounts = $derived(
    details ? Object.fromEntries(details.activityHistory.filter((item) => item.completedCount > 0).map((item) => [item.activityId, item.completedCount])) : {},
  );
  const selectedNode = $derived(details?.diagram.nodes.find((node) => node.id === selectedNodeId) ?? null);
  const canTeleport = $derived(details != null && details.activeActivities.some((activity) => activity.cancellable));

  const diagramProgress = $derived.by(() => {
    if (!details) return { completed: 0, total: 0, executions: 0, repeats: 0 };
    const completed = details.activityHistory.filter((item) => targetIds.includes(item.activityId) && item.completedCount > 0);
    return {
      completed: completed.length,
      total: targets.length,
      executions: completed.reduce((sum, item) => sum + item.completedCount, 0),
      repeats: completed.reduce((sum, item) => sum + Math.max(0, item.completedCount - 1), 0),
    };
  });

  const filteredVariables = $derived(
    details
      ? details.variables.filter((variable) => !variableQuery.trim() || contains(variable.name, variableQuery) || contains(variable.value, variableQuery) || contains(variable.type, variableQuery))
      : [],
  );

  const tabs: { id: Tab; label: string }[] = [
    { id: "diagram", label: "Схема" },
    { id: "timeline", label: "Хронология" },
    { id: "diagnosis", label: "Диагностика" },
    { id: "variables", label: "Переменные" },
    { id: "metadata", label: "Метаданные" },
  ];

  const EVIDENCE_LABEL: Record<CurrentActivityEvidence, { label: string; tone: string }> = {
    RuntimeActivity: { label: "АКТИВЕН", tone: "primary" },
    Transition: { label: "ПЕРЕХОД", tone: "warning" },
    UnfinishedHistory: { label: "НЕ ЗАВЕРШЁН", tone: "warning" },
    Incident: { label: "ИНЦИДЕНТ", tone: "danger" },
    Job: { label: "JOB", tone: "danger" },
    ExternalTask: { label: "EXTERNAL TASK", tone: "danger" },
  };

  function hasFailure(activity: ActiveActivityInstance): boolean {
    return activity.evidence.some((evidence) => evidence === "Incident" || evidence === "Job" || evidence === "ExternalTask");
  }

  function nodeFacts(node: BpmnNode) {
    const snapshot = details!;
    return {
      active: snapshot.activeActivities.some((activity) => activity.activityId === node.id),
      history: snapshot.activityHistory.find((item) => item.activityId === node.id) ?? null,
      incidents: snapshot.incidents.filter((incident) => (incident.failedActivityId ?? incident.activityId) === node.id),
      jobs: snapshot.jobs.filter((job) => job.failedActivityId === node.id),
      externalTasks: snapshot.externalTasks.filter((task) => task.activityId === node.id),
      canTeleport: targetIds.includes(node.id),
    };
  }
</script>

<div class="page">
  <div class="row head">
    <Button onclick={() => app.backFromProcessInstance()}>← Назад</Button>
    <div class="col grow">
      <div class="row">
        <h1 class="title">Заявка</h1>
        {#if details}<StateBadge suspended={details.instance.suspended} ended={details.instance.ended} labels="instance" />{/if}
      </div>
      <div class="row">
        <span class="mono muted small selectable">{processInstanceId}</span>
        <span class="small" class:primary={loading && details != null} class:muted={!(loading && details != null)}>
          {loading && details ? "Обновляем состояние…" : `Автообновление через: ${secondsUntilRefresh} сек.`}
        </span>
      </div>
    </div>
    <div class="icon-actions">
      <Button variant="ghost" loading={loading} disabled={operationBusy} onclick={refreshNow} title="Обновить (⌘R)"><Icon name="refresh" size={15} /></Button>
      <Button variant="ghost" disabled={!cockpitUrl} onclick={() => cockpitUrl && api.openInBrowser(cockpitUrl)} title="Открыть в Cockpit"><Icon name="external" size={15} /></Button>
      <Button variant="ghost" onclick={copyLink} title="Скопировать ссылку camunda-support://"><Icon name="link" size={15} /></Button>
      <Button variant="ghost" onclick={copyReport} disabled={!details} title="Скопировать отчёт в Markdown"><Icon name="file" size={15} /></Button>
      <Button variant="ghost" class={pinned ? "pinned" : ""} onclick={togglePin} title={pinned ? "Открепить" : "Закрепить в недавних"}><Icon name="star" size={15} /></Button>
    </div>
    {#if details}
      <Button disabled={operationBusy} onclick={() => (pendingAction = { kind: "suspension", suspended: !details!.instance.suspended })}>
        {details.instance.suspended ? "Активировать" : "Приостановить"}
      </Button>
      <Button variant="primary" disabled={operationBusy || !canTeleport} onclick={() => { teleportTargetId = null; teleportOpen = true; }}>Телепорт</Button>
    {/if}
  </div>

  {#if details}
    <div class="facts card row" in:rise>
      {#if details.instance.businessKey}
        <button type="button" class="fact link" onclick={() => copy(details!.instance.businessKey!, `Бизнес-ключ ${details!.instance.businessKey} скопирован.`, "Не удалось скопировать бизнес-ключ.")}>
          <span class="muted">Бизнес-ключ:</span> <span class="bold">{details.instance.businessKey}</span> <span class="tiny">⧉</span>
        </button>
      {:else}
        <span class="fact"><span class="muted">Бизнес-ключ:</span> <span class="muted">—</span></span>
      {/if}
      <span class="fact"><span class="muted">Процесс:</span> <span class="bold">{details.instance.definitionKey}</span></span>
      <span class="fact"><span class="muted">Текущих позиций:</span> <span class="bold">{details.activeActivities.length}</span></span>
      <span class="fact"><span class="muted">Инцидентов:</span> <span class="bold" class:danger={details.incidents.length > 0} class:healthy={details.incidents.length === 0}>{details.incidents.length}</span></span>
    </div>
  {/if}

  {#if operationError ?? entry.error}
    <div class="message message-danger selectable" transition:expand>{operationError ?? entry.error}</div>
  {/if}
  {#if operationMessage}
    <div class="message message-healthy selectable" transition:expand>{operationMessage}</div>
  {/if}

  {#if !details}
    {#if !entry.error}
      <div class="card shimmer skeleton"></div>
    {:else}
      <EmptyPanel title="Не удалось загрузить заявку" description={entry.error} actionLabel="Повторить" onaction={refreshNow} tone="danger" />
    {/if}
  {:else}
    <SegmentedTabs items={tabs} selected={tab} onselect={(id) => (tab = id)} tone="primary" />
    <div class="tab-host">
      {#key tab}
        <div class="tab-body" in:screenIn={{ duration: 200 }}>
          {#if tab === "diagram"}
            <div class="diagram-grid">
              <div class="card diagram-card">
                <div class="diagram-head">
                  <div class="row">
                    <span class="bold">Путь заявки</span>
                    <span class="grow"></span>
                    <span class="fact"><span class="muted">Пройдено:</span> <span class="healthy bold">{diagramProgress.completed} из {diagramProgress.total}</span></span>
                    <span class="fact"><span class="muted">Выполнений:</span> <span class="bold">{diagramProgress.executions}</span></span>
                    {#if diagramProgress.repeats > 0}<span class="fact"><span class="muted">Повторов:</span> <span class="warning bold">{diagramProgress.repeats}</span></span>{/if}
                  </div>
                  <div class="row legend">
                    <span class="pill pill-neutral">пройден</span>
                    <span class="pill pill-primary">активен</span>
                    <span class="pill pill-danger">инцидент</span>
                    <span class="faint grow truncate">Нажмите кубик для действий · Ctrl/⌘ + колесо — масштаб · колесо — прокрутка</span>
                  </div>
                </div>
                <div class="divider"></div>
                <div class="diagram-body">
                  {#if details.bpmnXml.trim() === ""}
                    <div class="center muted">Camunda не вернула BPMN XML</div>
                  {:else}
                    <BpmnViewer xml={details.bpmnXml} {activeIds} {incidentIds} {completedCounts} clickableIds={targetIds} selectedId={selectedNodeId} onActivityClick={(id) => (selectedNodeId = id)} />
                  {/if}
                </div>
              </div>
              <div class="side">
                <div class="card node-card" class:empty={!selectedNode}>
                  {#if !selectedNode}
                    <span class="eyebrow">Действия с кубиком</span>
                    <span class="bold">Нажмите на кубик в BPMN-схеме</span>
                    <span class="muted small">Покажем доступные действия именно для выбранного элемента.</span>
                  {:else}
                    {@const facts = nodeFacts(selectedNode)}
                    {#key selectedNode.id}
                      <div class="col gap8" in:rise={{ duration: 180 }}>
                        <span class="eyebrow">Выбранный кубик</span>
                        <span class="node-name clamp-2">{selectedNode.name ?? selectedNode.id}</span>
                        <div class="row wrap">
                          {#if facts.history && facts.history.completedCount > 0}<span class="pill pill-healthy">Пройден · ×{facts.history.completedCount}</span>{/if}
                          {#if facts.active}<span class="pill pill-primary">Активен</span>{/if}
                          {#if facts.incidents.length > 0}<span class="pill pill-danger">Ошибка · {facts.incidents.length}</span>{/if}
                          {#if facts.history && facts.history.canceledCount > 0}<span class="pill pill-warning">Отменён · {facts.history.canceledCount}</span>{/if}
                        </div>
                        <span class="mono faint tiny clamp-2 selectable">{[selectedNode.type, selectedNode.topic ? `topic: ${selectedNode.topic}` : null, selectedNode.id].filter(Boolean).join(" · ")}</span>
                        <div class="divider"></div>
                        <Button variant="primary" block disabled={operationBusy || !facts.canTeleport || !canTeleport} onclick={() => { teleportTargetId = selectedNode!.id; teleportOpen = true; }}>Телепортировать сюда</Button>
                        {#if facts.jobs[0]}
                          <Button block disabled={operationBusy} onclick={() => (pendingAction = { kind: "jobRetry", jobId: facts.jobs[0].id })}>Дать 3 попытки заданию</Button>
                        {/if}
                        {#if facts.externalTasks[0]}
                          <Button block disabled={operationBusy} onclick={() => (pendingAction = { kind: "externalRetry", taskId: facts.externalTasks[0].id })}>Дать 3 попытки внешней задаче</Button>
                          <Button block disabled={operationBusy || !facts.externalTasks[0].workerId} onclick={() => (pendingAction = { kind: "externalUnlock", taskId: facts.externalTasks[0].id })}>Разблокировать внешнюю задачу</Button>
                        {/if}
                        {#if !facts.canTeleport && facts.jobs.length === 0 && facts.externalTasks.length === 0}
                          <span class="muted small">Для этого элемента нет безопасных ручных операций.</span>
                        {/if}
                        {#if facts.jobs.length > 1 || facts.externalTasks.length > 1}
                          <span class="faint tiny">Другие задания этого кубика доступны во вкладке «Диагностика».</span>
                        {/if}
                      </div>
                    {/key}
                  {/if}
                </div>
                <div class="card panel">
                  <div class="row"><span class="panel-title">Текущее состояние</span><span class="grow"></span><span class="mono faint small">{details.activeActivities.length}</span></div>
                  <div class="divider"></div>
                  <div class="panel-body scroll">
                    {#each details.activeActivities as activity (activity.id)}
                      <div class="activity" animate:flip={flipOptions} in:rise>
                        <span class="activity-dot" class:danger-bg={hasFailure(activity)} class:primary-bg={!hasFailure(activity)}></span>
                        <div class="col grow">
                          <span class="small bold">{activity.activityName ?? activity.activityId}</span>
                          <span class="mono faint tiny truncate">{activity.activityId}</span>
                          <div class="row wrap">
                            {#each activity.evidence as evidence (evidence)}
                              <span class="pill pill-{EVIDENCE_LABEL[evidence].tone}">{EVIDENCE_LABEL[evidence].label}</span>
                            {/each}
                          </div>
                        </div>
                      </div>
                    {:else}
                      <span class="muted small">Активных элементов нет</span>
                    {/each}
                  </div>
                </div>
                <div class="card panel">
                  <div class="row"><span class="panel-title">Инциденты</span><span class="grow"></span><span class="mono faint small">{details.incidents.length}</span></div>
                  <div class="divider"></div>
                  <div class="panel-body scroll">
                    {#each details.incidents as incident (incident.id)}
                      <div class="incident selectable" animate:flip={flipOptions} in:rise>
                        <span class="danger bold small">{incident.failedActivityId ?? incident.activityId ?? incident.type}</span>
                        <span class="small">{incident.message ?? "Сообщение отсутствует"}</span>
                        <span class="mono faint tiny">
                          {[`тип: ${incident.type}`, incident.timestamp ? `время: ${incident.timestamp}` : null, incident.configuration ? `конфигурация: ${incident.configuration}` : null, incident.annotation ? `аннотация: ${incident.annotation}` : null, `id: ${incident.id}`].filter(Boolean).join("\n")}
                        </span>
                      </div>
                    {:else}
                      <span class="healthy small">Открытых инцидентов нет</span>
                    {/each}
                  </div>
                </div>
              </div>
            </div>
          {:else if tab === "timeline"}
            <div class="card section scroll timeline">
              <div class="row"><span class="panel-title">Хронология прохождения</span><span class="grow"></span><span class="faint small">{timeline.length} событий · History API</span></div>
              <div class="divider"></div>
              {#each timeline as entry, index (entry.id)}
                <div class="tl-row" class:canceled={entry.canceled} class:active-row={entry.endTime == null && !entry.canceled}>
                  <span class="tl-time mono faint">{formatTime(entry.startTime)}</span>
                  <span class="tl-dot" aria-hidden="true"></span>
                  <span class="col grow">
                    <span class="row">
                      <span class="small bold truncate">{entry.activityName ?? entry.activityId}</span>
                      <span class="faint tiny">{entry.activityType}</span>
                      {#if entry.canceled}<span class="pill pill-warning">отменён</span>{/if}
                      {#if entry.endTime == null && !entry.canceled}<span class="pill pill-primary">активен</span>{/if}
                    </span>
                    <span class="mono faint tiny truncate selectable">{entry.activityId}</span>
                  </span>
                  <span class="mono small tl-duration" class:muted={entry.durationMillis != null}>{formatShort(entry.durationMillis)}</span>
                  <span class="faint tiny tl-index">#{index + 1}</span>
                </div>
              {:else}
                <span class="muted">История активностей недоступна</span>
              {/each}
            </div>
          {:else if tab === "diagnosis"}
            <div class="diagnosis scroll">
              <div class="card section">
                <div class="row"><span class="panel-title">Инциденты</span><span class="grow"></span><span class="faint small">{details.incidents.length}</span></div>
                <div class="divider"></div>
                {#each details.incidents as incident (incident.id)}
                  <div class="incident selectable">
                    <span class="danger bold small">{incident.failedActivityId ?? incident.activityId ?? incident.type}</span>
                    <span class="small">{incident.message ?? "Сообщение отсутствует"}</span>
                    <span class="mono faint tiny">{[`тип: ${incident.type}`, incident.timestamp ? `время: ${incident.timestamp}` : null, incident.configuration ? `конфигурация: ${incident.configuration}` : null, incident.annotation ? `аннотация: ${incident.annotation}` : null, `id: ${incident.id}`].filter(Boolean).join("\n")}</span>
                  </div>
                {:else}
                  <span class="healthy">Открытых инцидентов нет</span>
                {/each}
              </div>
              <div class="card section">
                <div class="row"><span class="panel-title">Задания движка</span><span class="grow"></span><span class="faint small">{details.jobs.length}</span></div>
                <div class="divider"></div>
                {#each details.jobs as job (job.id)}
                  <div class="row job">
                    <div class="col grow selectable">
                      <span class="small bold">Задание · осталось попыток: {job.retries}</span>
                      <span class="small clamp-2" class:danger={job.exceptionMessage != null} class:muted={job.exceptionMessage == null}>{job.exceptionMessage ?? "Ошибка отсутствует"}</span>
                      <span class="mono faint tiny">{job.failedActivityId ?? job.id}</span>
                    </div>
                    <Button variant="primary" size="sm" disabled={operationBusy} onclick={() => (pendingAction = { kind: "jobRetry", jobId: job.id })}>Дать 3 попытки</Button>
                  </div>
                {:else}
                  <span class="muted">Задания движка не найдены</span>
                {/each}
              </div>
              <div class="card section">
                <div class="row"><span class="panel-title">Внешние задачи</span><span class="grow"></span><span class="faint small">{details.externalTasks.length}</span></div>
                <div class="divider"></div>
                {#each details.externalTasks as task (task.id)}
                  <div class="row job">
                    <div class="col grow selectable">
                      <span class="small bold">{task.topicName} · попыток: {task.retries ?? "—"}</span>
                      <span class="small clamp-2" class:danger={task.errorMessage != null} class:muted={task.errorMessage == null}>{task.errorMessage ?? "Ошибка отсутствует"}</span>
                      <span class="faint tiny">обработчик: {task.workerId ?? "—"} · блокировка до: {task.lockExpirationTime ?? "—"}</span>
                    </div>
                    <Button variant="primary" size="sm" disabled={operationBusy} onclick={() => (pendingAction = { kind: "externalRetry", taskId: task.id })}>Дать 3 попытки</Button>
                    <Button size="sm" disabled={operationBusy || !task.workerId} onclick={() => (pendingAction = { kind: "externalUnlock", taskId: task.id })}>Разблокировать</Button>
                  </div>
                {:else}
                  <span class="muted">Внешние задачи не найдены</span>
                {/each}
              </div>
            </div>
          {:else if tab === "variables"}
            <div class="variables">
              <div class="row">
                <span class="bold grow">Переменные Camunda</span>
                <span class="muted small">{filteredVariables.length} из {details.variables.length}</span>
                <TextField class="var-search" size="sm" placeholder="Поиск по ключу, типу или значению" bind:value={variableQuery} type="search" />
              </div>
              <div class="var-list scroll">
                {#each filteredVariables as variable (variable.name)}
                  <div class="card variable row" animate:flip={flipOptions}>
                    <div class="col var-name">
                      <span class="mono bold small">{variable.name}</span>
                      <span class="faint tiny">{variable.type}</span>
                    </div>
                    <span class="mono tiny grow var-value selectable">{variable.value}</span>
                    <Button size="sm" onclick={() => copy(variable.value, `Значение переменной ${variable.name} скопировано.`, "Не удалось скопировать значение переменной.")}>Копировать</Button>
                    <Button size="sm" disabled={operationBusy || !isVariableEditable(variable)} onclick={() => { variableEditError = null; variableToEdit = variable; }}>
                      {isVariableEditable(variable) ? "Изменить" : "Только чтение"}
                    </Button>
                  </div>
                {:else}
                  <EmptyPanel title="Переменные не найдены" description="Измените поисковый запрос." compact />
                {/each}
              </div>
            </div>
          {:else}
            <div class="card section scroll">
              <div class="row"><span class="panel-title">Метаданные заявки</span><span class="grow"></span><span class="faint small">{details.metadata.length} полей</span></div>
              <div class="divider"></div>
              <div class="metadata selectable">
                {#each details.metadata as field (field.label)}
                  <span class="muted small">{field.label}</span>
                  <span class="mono small">{field.value}</span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/key}
    </div>
  {/if}
</div>

<ConfirmDialog open={pendingAction != null} description={pendingAction ? actionDescription(pendingAction) : ""} {production} busy={operationBusy} onconfirm={() => pendingAction && execute(pendingAction)} oncancel={() => (pendingAction = null)} />

{#if details}
  <TeleportDialog open={teleportOpen} {details} initialTargetId={teleportTargetId} {production} busy={operationBusy} onexecute={executeTeleport} onclose={() => !operationBusy && (teleportOpen = false)} />
{/if}

<VariableEditDialog variable={variableToEdit} {production} busy={operationBusy} error={variableEditError} onsave={saveVariable} onclose={() => !operationBusy && (variableToEdit = null)} />

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    height: 100%;
  }
  .head {
    gap: 8px;
  }
  .icon-actions {
    display: flex;
    gap: 2px;
  }
  .icon-actions :global(.pinned) {
    color: var(--warning);
  }
  .title {
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 1.2;
  }
  .small {
    font-size: 11px;
  }
  .tiny {
    font-size: 9.5px;
  }
  .bold {
    font-weight: 650;
  }
  .primary { color: var(--accent); }
  .healthy { color: var(--healthy); }
  .danger { color: var(--danger); }
  .warning { color: var(--warning); }
  .facts {
    padding: 8px 12px;
    gap: 20px;
  }
  .fact {
    font-size: 11px;
    display: inline-flex;
    gap: 5px;
    align-items: baseline;
    white-space: nowrap;
  }
  .fact.link {
    color: var(--ink);
  }
  .fact.link .bold {
    color: var(--accent);
  }
  .skeleton {
    flex: 1;
  }
  .tab-host {
    flex: 1;
    min-height: 0;
    display: grid;
  }
  .tab-body {
    grid-area: 1 / 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .diagram-grid {
    display: grid;
    grid-template-columns: 1fr 320px;
    gap: 10px;
    flex: 1;
    min-height: 0;
  }
  .diagram-card {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  .diagram-head {
    padding: 9px 14px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
  }
  .legend {
    font-size: 10.5px;
    font-weight: 600;
    gap: 12px;
  }
  .diagram-body {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  .center {
    display: grid;
    place-items: center;
    height: 100%;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }
  .node-card {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-color: var(--accent);
    flex: none;
  }
  .node-card.empty {
    background: var(--accent-soft);
    border-color: var(--border);
  }
  .node-name {
    font-size: 13px;
    font-weight: 650;
  }
  .gap8 {
    gap: 8px;
  }
  .wrap {
    flex-wrap: wrap;
    gap: 4px;
  }
  .panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    min-height: 0;
  }
  .panel-title {
    font-size: 12px;
    font-weight: 600;
  }
  .panel-body {
    display: flex;
    flex-direction: column;
    gap: 7px;
    flex: 1;
    min-height: 0;
  }
  .activity {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .activity-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    margin-top: 5px;
    flex: none;
  }
  .primary-bg { background: var(--accent); }
  .danger-bg { background: var(--danger); }
  .incident {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 9px;
    border-radius: 7px;
    background: var(--danger-soft);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .diagnosis {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-right: 4px;
  }
  .diagnosis > :global(*) {
    flex-shrink: 0;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 9px;
    padding: 14px;
  }
  .job {
    gap: 8px;
    padding: 3px 0;
  }
  .variables {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    min-height: 0;
  }
  .variables :global(.var-search) {
    width: 360px;
  }
  .var-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-height: 0;
    padding-right: 4px;
  }
  .variable {
    padding: 9px 12px;
    gap: 10px;
    align-items: flex-start;
  }
  .var-name {
    width: 240px;
    flex: none;
  }
  .var-value {
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 120px;
    overflow: auto;
  }
  .timeline {
    gap: 0;
  }
  .timeline .divider {
    margin-bottom: 6px;
  }
  .tl-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 4px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
    position: relative;
  }
  .tl-row:last-child {
    border-bottom: 0;
  }
  .tl-time {
    width: 92px;
    font-size: 10.5px;
    flex: none;
  }
  .tl-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--healthy);
    flex: none;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--healthy) 18%, transparent);
  }
  .canceled .tl-dot {
    background: var(--warning);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--warning) 18%, transparent);
  }
  .active-row .tl-dot {
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 22%, transparent);
    animation: pulse-dot 2s ease-in-out infinite;
  }
  .tl-duration {
    width: 70px;
    text-align: right;
    flex: none;
  }
  .active-row .tl-duration {
    color: var(--accent);
  }
  .tl-index {
    width: 32px;
    text-align: right;
    flex: none;
  }
  .metadata {
    display: grid;
    grid-template-columns: 250px 1fr;
    gap: 6px 12px;
  }
</style>
