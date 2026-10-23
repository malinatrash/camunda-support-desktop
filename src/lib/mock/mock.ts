import bpmnXml from "./process.bpmn?raw";
import type { api as realApi } from "../api";
import type {
  CamundaConnection,
  ProcessDashboard,
  ProcessDefinitionSummary,
  ProcessIncident,
  ProcessInstanceDetails,
  ProcessInstanceSummary,
} from "../types";

// Демо-данные для запуска интерфейса в обычном браузере без Tauri (pnpm dev).
const delay = (ms = 350) => new Promise((resolve) => setTimeout(resolve, ms));

let connections: CamundaConnection[] = [
  { id: "b2c-prod", name: "B2C Prod", restUrl: "https://camunda-b2c.example.kz/engine-rest", cockpitUrl: "https://camunda-b2c.example.kz/camunda/app/cockpit", environment: "Production", engineVersion: "7.17.0" },
  { id: "pledge-test", name: "Залоги TEST", restUrl: "https://camunda-pledge.test.example.kz/engine-rest", cockpitUrl: "https://camunda-pledge.test.example.kz/camunda/app/cockpit", environment: "Test", engineVersion: "7.17.0" },
  { id: "wpl-dev", name: "WPL DEV", restUrl: "http://camunda-wpl.dev.internal/engine-rest", cockpitUrl: "http://camunda-wpl.dev.internal/camunda/app/cockpit", environment: "Development", engineVersion: "7.24.0" },
];

function definition(key: string, name: string, version: number, instances: number, incidents: number, extra: Partial<ProcessDefinitionSummary> = {}): ProcessDefinitionSummary {
  return {
    id: `${key}:${version}:${key.length}f3a-${version}`,
    key,
    name,
    version,
    tenantId: null,
    suspended: false,
    deploymentId: `dep-${key}-${version}`,
    instances,
    incidents,
    versionTag: null,
    category: "http://bpmn.io/schema/bpmn",
    description: null,
    resource: `${key}.bpmn`,
    diagramResource: null,
    historyTimeToLive: 30,
    startableInTasklist: true,
    ...extra,
  };
}

const definitions = [
  definition("drzi", "Доверенность ЭЦП", 12, 148, 7, { versionTag: "2.4.1", description: "Выпуск доверенности с подписанием через ЭЦП" }),
  definition("pledge-loan", "Залоговый кредит", 31, 912, 2),
  definition("auto-online", "Автокредит онлайн", 8, 240, 0, { tenantId: "b2c" }),
  definition("share-eds", "Биометрия и ЭЦП", 5, 54, 0, { suspended: true }),
  definition("wpl", "Единый лимит", 3, 17, 1, { versionTag: "wpl-800" }),
];

function timeline(points: number) {
  return Array.from({ length: points }, (_, index) => ({
    label: `23.10 ${String(index).padStart(2, "0")}:00`,
    started: Math.round(20 + 18 * Math.sin(index / 3) + (index % 5) * 3),
    completed: Math.round(14 + 15 * Math.sin(index / 3 - 0.6) + (index % 4) * 2),
    incidents: index % 7 === 3 ? 4 : index % 5 === 0 ? 1 : 0,
  }));
}

const dashboard: ProcessDashboard = {
  definitions,
  timeline: timeline(24),
  incidentTypes: [
    { type: "failedExternalTask", count: 7 },
    { type: "failedJob", count: 3 },
  ],
  incidentActivities: [
    { activityId: "Activity_0soegtg", processDefinitionId: definitions[0].id, count: 5, activityName: "Подготовить задачу", topic: "prepare-task" },
    { activityId: "Activity_approve", processDefinitionId: definitions[0].id, count: 2, activityName: "Согласовать", topic: null },
    { activityId: "Task_score", processDefinitionId: definitions[1].id, count: 2, activityName: "Скоринг", topic: "scoring" },
    { activityId: "Task_limit", processDefinitionId: definitions[4].id, count: 1, activityName: "Расчёт лимита", topic: "limit" },
  ],
  longestWaitingActivities: [
    { activityId: "Activity_approve", activityName: "Согласовать", activityType: "userTask", topic: null, processDefinitionId: definitions[0].id, oldestProcessInstanceId: "9d1f6e2a-0001", instanceCount: 23, averageWaitingMillis: 5 * 3_600_000, maximumWaitingMillis: 31 * 3_600_000, oldestStartedAt: "2026-10-22T09:12:44.000+0500" },
    { activityId: "Task_score", activityName: "Скоринг", activityType: "serviceTask", topic: "scoring", processDefinitionId: definitions[1].id, oldestProcessInstanceId: "9d1f6e2a-0002", instanceCount: 8, averageWaitingMillis: 40 * 60_000, maximumWaitingMillis: 6 * 3_600_000, oldestStartedAt: "2026-10-23T08:02:11.000+0500" },
    { activityId: "Task_sign", activityName: "Подписание", activityType: "receiveTask", topic: null, processDefinitionId: definitions[2].id, oldestProcessInstanceId: "9d1f6e2a-0003", instanceCount: 41, averageWaitingMillis: 12 * 60_000, maximumWaitingMillis: 55 * 60_000, oldestStartedAt: "2026-10-23T12:40:00.000+0500" },
  ],
  durationBuckets: [
    { label: "< 1 мин", count: 120 },
    { label: "1–5 мин", count: 310 },
    { label: "5–15 мин", count: 180 },
    { label: "15–60 мин", count: 95 },
    { label: "1–4 ч", count: 40 },
    { label: "4–24 ч", count: 12 },
    { label: "> 24 ч", count: 6 },
  ],
  timelineTitle: "Динамика за последние 24 часа",
  averageDurationMillis: 23 * 60_000,
  medianDurationMillis: 6 * 60_000,
  p95DurationMillis: 3 * 3_600_000,
  completedInstances: 763,
  timelineTruncated: false,
  incidents: [],
};

function instance(id: string, businessKey: string | null, suspended = false): ProcessInstanceSummary {
  return { id, definitionId: definitions[0].id, definitionKey: "drzi", businessKey, suspended, tenantId: null, ended: false, startTime: "2026-10-23T10:15:00.000+0500", endTime: null, state: "ACTIVE", definitionName: "Доверенность ЭЦП" };
}

const instances = Array.from({ length: 60 }, (_, index) => instance(`9d1f6e2a-${String(index + 1).padStart(4, "0")}`, index % 3 === 0 ? null : `APP-${10200 + index}`, index % 9 === 4));

const incidents: ProcessIncident[] = Array.from({ length: 9 }, (_, index) => ({
  id: `inc-${index + 1}`,
  type: index % 3 === 0 ? "failedJob" : "failedExternalTask",
  activityId: "Activity_0soegtg",
  failedActivityId: "Activity_0soegtg",
  message: index % 2 === 0 ? "ENGINE-03004 Exception while executing external task: Connection refused (prepare-task)" : "java.net.SocketTimeoutException: Read timed out after 30000 ms while calling DRZI service",
  configuration: index % 3 === 0 ? `job-${index}` : `ext-${index}`,
  timestamp: `2026-10-23T1${index}:05:33.000+0500`,
  processDefinitionId: definitions[0].id,
  processInstanceId: instances[index].id,
  executionId: `exec-${index}`,
  tenantId: null,
  annotation: index === 2 ? "Повтор после фикса сети" : null,
}));

function details(processInstanceId: string): ProcessInstanceDetails {
  const summary = instances.find((candidate) => candidate.id === processInstanceId) ?? instance(processInstanceId, "APP-99999");
  return {
    instance: summary,
    variables: [
      { name: "applicationId", type: "String", value: summary.businessKey ?? "APP-99999", valueInfo: null },
      { name: "borrower_iin", type: "String", value: "880101300123", valueInfo: null },
      { name: "amount", type: "Long", value: "2500000", valueInfo: null },
      { name: "approved", type: "Boolean", value: "false", valueInfo: null },
      { name: "payload", type: "Json", value: '{"channel":"mobile","attempt":2,"documents":["id","income"]}', valueInfo: null },
      { name: "document", type: "Object", value: "<serialized>", valueInfo: '{"objectTypeName":"kz.bereke.Document"}' },
    ],
    activeActivities: [
      { id: "ai-1", activityId: "Activity_0soegtg", activityName: "Подготовить задачу", activityType: "serviceTask", executionIds: ["exec-1"], incidentIds: ["inc-1"], evidence: ["RuntimeActivity", "Incident", "ExternalTask"], cancellable: true },
    ],
    activityHistory: [
      { activityId: "StartEvent_root", activityName: "Старт", activityType: "startEvent", completedCount: 1, activeCount: 0, canceledCount: 0 },
      { activityId: "Activity_1gooqz2", activityName: "Подготовка", activityType: "subProcess", completedCount: 0, activeCount: 1, canceledCount: 0 },
      { activityId: "StartEvent_prepare", activityName: null, activityType: "startEvent", completedCount: 2, activeCount: 0, canceledCount: 0 },
      { activityId: "Activity_0soegtg", activityName: "Подготовить задачу", activityType: "serviceTask", completedCount: 1, activeCount: 1, canceledCount: 0 },
    ],
    incidents: incidents.slice(0, 1),
    jobs: [{ id: "job-1", retries: 0, failedActivityId: "Activity_0soegtg", exceptionMessage: "ENGINE-03004 Exception while executing external task", dueDate: null, suspended: false }],
    externalTasks: [{ id: "ext-1", activityId: "Activity_0soegtg", topicName: "prepare-task", retries: 0, errorMessage: "Connection refused", workerId: "worker-7", lockExpirationTime: "2026-10-23T12:00:00.000+0500", suspended: false }],
    diagram: {
      nodes: [
        { id: "StartEvent_root", name: "Старт", type: "startEvent", x: 0, y: 0, width: 36, height: 36, topic: null },
        { id: "Activity_1gooqz2", name: "Подготовка", type: "subProcess", x: 0, y: 0, width: 100, height: 80, topic: null },
        { id: "Activity_0soegtg", name: "Подготовить задачу", type: "serviceTask", x: 0, y: 0, width: 100, height: 80, topic: "prepare-task" },
        { id: "Activity_1xqooxf", name: "Согласование", type: "subProcess", x: 0, y: 0, width: 100, height: 80, topic: null },
        { id: "Activity_approve", name: "Согласовать", type: "userTask", x: 0, y: 0, width: 100, height: 80, topic: null },
        { id: "EndEvent_root", name: "Конец", type: "endEvent", x: 0, y: 0, width: 36, height: 36, topic: null },
      ],
      edges: [],
    },
    bpmnXml,
    activityTimeline: [
      { id: "h1", activityId: "StartEvent_root", activityName: "Старт", activityType: "startEvent", startTime: "2026-10-23T10:15:00.000+0500", endTime: "2026-10-23T10:15:00.120+0500", durationMillis: 120, canceled: false },
      { id: "h2", activityId: "Activity_1gooqz2", activityName: "Подготовка", activityType: "subProcess", startTime: "2026-10-23T10:15:00.200+0500", endTime: null, durationMillis: null, canceled: false },
      { id: "h3", activityId: "StartEvent_prepare", activityName: null, activityType: "startEvent", startTime: "2026-10-23T10:15:00.300+0500", endTime: "2026-10-23T10:15:00.310+0500", durationMillis: 10, canceled: false },
      { id: "h4", activityId: "Activity_0soegtg", activityName: "Подготовить задачу", activityType: "serviceTask", startTime: "2026-10-23T10:15:01.000+0500", endTime: "2026-10-23T10:16:40.000+0500", durationMillis: 99_000, canceled: true },
      { id: "h5", activityId: "Activity_0soegtg", activityName: "Подготовить задачу", activityType: "serviceTask", startTime: "2026-10-23T10:17:00.000+0500", endTime: null, durationMillis: null, canceled: false },
    ],
    metadata: [
      { label: "ID экземпляра", value: summary.id },
      { label: "ID определения", value: summary.definitionId },
      { label: "Ключ определения", value: "drzi" },
      { label: "Бизнес-ключ", value: summary.businessKey ?? "—" },
      { label: "ID экземпляра кейса", value: "—" },
      { label: "ID тенанта", value: "—" },
      { label: "Завершён", value: "false" },
      { label: "Приостановлен", value: String(summary.suspended) },
    ],
  };
}

export const mockApi: typeof realApi = {
  appInfo: async () => ({ version: "2.0.0-demo", build: "browser", platform: "browser" }),
  listConnections: async () => connections,
  addConnection: async (draft, engineVersion) => {
    await delay();
    const connection: CamundaConnection = { id: draft.name.toLowerCase().replace(/\W+/g, "-"), name: draft.name, restUrl: draft.restUrl, cockpitUrl: draft.cockpitUrl, environment: draft.environment, engineVersion };
    connections = [...connections, connection];
    return connection;
  },
  deleteConnection: async (connectionId) => {
    connections = connections.filter((connection) => connection.id !== connectionId);
  },
  validateConnectionDraft: async () => ({ nameError: null, restUrlError: null, cockpitUrlError: null }),
  deriveCockpitUrl: async (restUrl) => restUrl.replace(/\/engine-rest\/?$/, "/camunda/app/cockpit"),
  testConnection: async () => {
    await delay(900);
    return { engineVersion: "7.17.0" };
  },
  getTheme: async () => (localStorage.getItem("mock-theme") as "Light" | "Dark" | null) ?? "Light",
  setTheme: async (theme) => localStorage.setItem("mock-theme", theme),
  loadDashboard: async (_connection, dateFilter) => {
    await delay(500);
    return { ...dashboard, incidents, timelineTitle: dateFilter.preset === "Live" ? dashboard.timelineTitle : `Динамика за ${dateFilter.preset.toLowerCase()}` };
  },
  loadIncidents: async () => {
    await delay(400);
    return incidents;
  },
  loadProcessDefinitionVersions: async () => definitions.slice(0, 1).map((d) => ({ ...d })),
  searchProcessInstances: async () => {
    await delay(600);
    return instances.slice(0, 25);
  },
  loadSearchProcessDefinitions: async () => {
    await delay(300);
    return definitions;
  },
  loadProcessVariableCatalog: async (_c, key) => {
    await delay(500);
    return { processDefinitionKey: key, variables: [{ name: "applicationId", types: ["String"], occurrences: 60 }, { name: "borrower_iin", types: ["String"], occurrences: 58 }, { name: "amount", types: ["Long", "Double"], occurrences: 60 }], inspectedInstanceCount: 60, instancesTruncated: false };
  },
  invalidateProcessVariableCatalog: async () => undefined,
  loadProcessDefinitionDetails: async (_c, processDefinitionId, dateFilter) => {
    await delay(400);
    const def = definitions.find((candidate) => candidate.id === processDefinitionId) ?? definitions[0];
    return { definition: def, instances: instances.map((item, index) => ({ instance: item, incidentCount: index % 7 === 0 ? 1 : 0 })), metadata: [{ label: "ID определения", value: def.id }, { label: "Ключ", value: def.key }, { label: "Версия", value: String(def.version) }], dateFilter };
  },
  loadProcessInstanceDetails: async (_c, processInstanceId) => {
    await delay(500);
    return details(processInstanceId);
  },
  searchInstancesByBusinessKey: async (_c, query) => {
    await delay(350);
    return instances.filter((item) => item.businessKey?.includes(query) || item.id.includes(query)).slice(0, 10);
  },
  retryIncidents: async (_c, list) => {
    await delay(700);
    return { jobs: list.filter((i) => i.type === "failedJob").length, externalTasks: list.filter((i) => i.type === "failedExternalTask").length, skipped: 0 };
  },
  teleportProcessInstance: async () => delay(800).then(() => undefined),
  setJobRetries: async () => delay(500).then(() => undefined),
  setExternalTaskRetries: async () => delay(500).then(() => undefined),
  unlockExternalTask: async () => delay(500).then(() => undefined),
  setProcessInstanceSuspended: async () => delay(500).then(() => undefined),
  updateProcessVariable: async () => delay(600).then(() => undefined),
  checkForUpdate: async () => ({ kind: "available", update: { version: "2.1.0", tag: "v2.1.0", releaseUrl: "https://github.com/malinatrash/camunda-support-desktop/releases/tag/v2.1.0", notes: "", asset: { name: "Camunda.Support_2.1.0_aarch64.dmg", downloadUrl: "https://example.test/app.dmg", sizeBytes: 9_400_000, digest: null }, checksumUrl: "https://example.test/SHA256SUMS.txt" } }),
  downloadAndOpenUpdate: async (_update, onProgress) => {
    for (let step = 0; step <= 10; step += 1) {
      await delay(180);
      onProgress({ stage: step < 10 ? "Downloading" : "Verifying", downloadedBytes: 940_000 * step, totalBytes: 9_400_000, bytesPerSecond: 5_200_000, destinationPath: "/tmp/camunda-support-updates/v2.1.0/Camunda.Support_2.1.0_aarch64.dmg" });
    }
    await delay(300);
    return { kind: "installerOpened", fileName: "Camunda.Support_2.1.0_aarch64.dmg" };
  },
  loadReleaseDownloadStats: async () => {
    await delay(700);
    return { totalDownloads: 1284, macOsDownloads: 812, windowsDownloads: 401, linuxDownloads: 71, releaseCount: 12, installerAssetCount: 46 };
  },
  parseDeepLink: async (raw) => (raw.includes("process-instance") ? { kind: "openProcessInstance", link: { origin: "https://camunda-b2c.example.kz", processInstanceId: raw.split("/").pop() ?? "x" } } : { kind: "invalid", message: "Ссылка не относится к заявке Camunda." }),
  processInstanceDeepLink: async (_c, id) => `camunda-support://process-instance/${id}?origin=https%3A%2F%2Fcamunda-b2c.example.kz`,
  takePendingDeepLinks: async () => [],
  openInBrowser: async (url) => {
    window.open(url, "_blank");
  },
};
