export type Environment = "Production" | "Test" | "Development" | "Local";

export interface CamundaConnection {
  id: string;
  name: string;
  restUrl: string;
  cockpitUrl: string;
  environment: Environment;
  engineVersion: string | null;
}

export interface ConnectionDraft {
  name: string;
  restUrl: string;
  cockpitUrl: string;
  environment: Environment;
}

export interface ConnectionValidation {
  nameError: string | null;
  restUrlError: string | null;
  cockpitUrlError: string | null;
}

export type AppThemeMode = "Light" | "Dark";
export type VariableValueType = "Auto" | "String" | "Number" | "Boolean";
export type DashboardDatePreset = "Live" | "Today" | "Yesterday" | "Custom";

export interface DashboardDateFilter {
  preset: DashboardDatePreset;
  fromDate: string;
  toDate: string;
}

export const LIVE_FILTER: DashboardDateFilter = { preset: "Live", fromDate: "", toDate: "" };

export interface ProcessDefinitionSummary {
  id: string;
  key: string;
  name: string | null;
  version: number;
  tenantId: string | null;
  suspended: boolean;
  deploymentId: string | null;
  instances: number;
  incidents: number;
  versionTag: string | null;
  category: string | null;
  description: string | null;
  resource: string | null;
  diagramResource: string | null;
  historyTimeToLive: number | null;
  startableInTasklist: boolean;
}

export interface DashboardTimelinePoint {
  label: string;
  started: number;
  completed: number;
  incidents: number;
}

export interface DashboardIncidentType {
  type: string;
  count: number;
}

export interface DashboardIncidentActivity {
  activityId: string;
  processDefinitionId: string | null;
  count: number;
  activityName: string | null;
  topic: string | null;
}

export interface DashboardDurationBucket {
  label: string;
  count: number;
}

export interface DashboardWaitingActivity {
  activityId: string;
  activityName: string | null;
  activityType: string;
  topic: string | null;
  processDefinitionId: string;
  oldestProcessInstanceId: string;
  instanceCount: number;
  averageWaitingMillis: number;
  maximumWaitingMillis: number;
  oldestStartedAt: string | null;
}

export interface ProcessDashboard {
  definitions: ProcessDefinitionSummary[];
  timeline: DashboardTimelinePoint[];
  incidentTypes: DashboardIncidentType[];
  incidentActivities: DashboardIncidentActivity[];
  longestWaitingActivities: DashboardWaitingActivity[];
  durationBuckets: DashboardDurationBucket[];
  timelineTitle: string;
  averageDurationMillis: number | null;
  medianDurationMillis: number | null;
  p95DurationMillis: number | null;
  completedInstances: number;
  timelineTruncated: boolean;
  incidents: ProcessIncident[];
}

export interface ProcessInstanceSummary {
  id: string;
  definitionId: string;
  definitionKey: string;
  businessKey: string | null;
  suspended: boolean;
  tenantId: string | null;
  ended: boolean;
  startTime: string | null;
  endTime: string | null;
  state: string | null;
  definitionName: string | null;
}

export interface RuntimeInstanceListItem {
  instance: ProcessInstanceSummary;
  incidentCount: number;
}

export interface ProcessMetadataField {
  label: string;
  value: string;
}

export interface ProcessDefinitionDetails {
  definition: ProcessDefinitionSummary;
  instances: RuntimeInstanceListItem[];
  metadata: ProcessMetadataField[];
  dateFilter: DashboardDateFilter;
}

export interface ProcessVariable {
  name: string;
  type: string;
  value: string;
  valueInfo: string | null;
}

export interface ProcessVariableDescriptor {
  name: string;
  types: string[];
  occurrences: number;
}

export interface ProcessVariableCatalog {
  processDefinitionKey: string;
  variables: ProcessVariableDescriptor[];
  inspectedInstanceCount: number;
  instancesTruncated: boolean;
}

export interface ProcessVariableUpdate {
  name: string;
  type: string;
  value: string;
  valueInfo: string | null;
}

export type CurrentActivityEvidence =
  | "RuntimeActivity"
  | "Transition"
  | "UnfinishedHistory"
  | "Incident"
  | "Job"
  | "ExternalTask";

export interface ActiveActivityInstance {
  id: string;
  activityId: string;
  activityName: string | null;
  activityType: string;
  executionIds: string[];
  incidentIds: string[];
  evidence: CurrentActivityEvidence[];
  cancellable: boolean;
}

export interface ActivityExecutionSummary {
  activityId: string;
  activityName: string | null;
  activityType: string;
  completedCount: number;
  activeCount: number;
  canceledCount: number;
}

export interface ProcessIncident {
  id: string;
  type: string;
  activityId: string | null;
  failedActivityId: string | null;
  message: string | null;
  timestamp: string | null;
  configuration: string | null;
  processDefinitionId: string | null;
  processInstanceId: string | null;
  executionId: string | null;
  tenantId: string | null;
  annotation: string | null;
}

export interface ProcessJob {
  id: string;
  retries: number;
  failedActivityId: string | null;
  exceptionMessage: string | null;
  dueDate: string | null;
  suspended: boolean;
}

export interface ProcessExternalTask {
  id: string;
  activityId: string;
  topicName: string;
  retries: number | null;
  errorMessage: string | null;
  workerId: string | null;
  lockExpirationTime: string | null;
  suspended: boolean;
}

export interface BpmnNode {
  id: string;
  name: string | null;
  type: string;
  x: number;
  y: number;
  width: number;
  height: number;
  topic: string | null;
}

export interface BpmnEdge {
  id: string;
  sourceRef: string | null;
  targetRef: string | null;
  waypoints: { x: number; y: number }[];
}

export interface BpmnDiagram {
  nodes: BpmnNode[];
  edges: BpmnEdge[];
}

export const TELEPORT_TARGET_TYPES = new Set([
  "task",
  "userTask",
  "serviceTask",
  "scriptTask",
  "manualTask",
  "businessRuleTask",
  "sendTask",
  "receiveTask",
  "callActivity",
  "subProcess",
  "intermediateCatchEvent",
  "intermediateThrowEvent",
  "exclusiveGateway",
  "parallelGateway",
  "inclusiveGateway",
  "eventBasedGateway",
]);

export function teleportTargets(diagram: BpmnDiagram): BpmnNode[] {
  return diagram.nodes.filter((node) => TELEPORT_TARGET_TYPES.has(node.type));
}

export interface ProcessInstanceDetails {
  instance: ProcessInstanceSummary;
  variables: ProcessVariable[];
  activeActivities: ActiveActivityInstance[];
  activityHistory: ActivityExecutionSummary[];
  incidents: ProcessIncident[];
  jobs: ProcessJob[];
  externalTasks: ProcessExternalTask[];
  diagram: BpmnDiagram;
  bpmnXml: string;
  metadata: ProcessMetadataField[];
  activityTimeline: ActivityTimelineEntry[];
}

export interface ActivityTimelineEntry {
  id: string;
  activityId: string;
  activityName: string | null;
  activityType: string;
  startTime: string | null;
  endTime: string | null;
  durationMillis: number | null;
  canceled: boolean;
}

export interface BulkRetryResult {
  jobs: number;
  externalTasks: number;
  skipped: number;
}

export interface TeleportRequest {
  sourceActivityInstanceId: string;
  targetActivityId: string;
  annotation: string;
  skipCustomListeners: boolean;
  skipIoMappings: boolean;
}

export interface ConnectionTestSuccess {
  engineVersion: string | null;
}

export interface AppUpdateAsset {
  name: string;
  downloadUrl: string;
  sizeBytes: number;
  digest: string | null;
}

export interface AppUpdate {
  version: string;
  tag: string;
  releaseUrl: string;
  notes: string;
  asset: AppUpdateAsset | null;
  checksumUrl: string | null;
}

export type UpdateCheckResult =
  | { kind: "upToDate" }
  | { kind: "available"; update: AppUpdate }
  | { kind: "failure"; message: string };

export type UpdateInstallResult =
  | { kind: "installerOpened"; fileName: string }
  | { kind: "failure"; message: string };

export type UpdateDownloadStage = "Preparing" | "Downloading" | "Verifying" | "OpeningInstaller";

export interface UpdateDownloadProgress {
  stage: UpdateDownloadStage;
  downloadedBytes: number;
  totalBytes: number;
  bytesPerSecond: number;
  destinationPath: string;
}

export interface ReleaseDownloadStats {
  totalDownloads: number;
  macOsDownloads: number;
  windowsDownloads: number;
  linuxDownloads: number;
  releaseCount: number;
  installerAssetCount: number;
}

export interface ProcessInstanceDeepLink {
  origin: string;
  processInstanceId: string;
}

export type DeepLinkEvent =
  | { kind: "openProcessInstance"; link: ProcessInstanceDeepLink }
  | { kind: "invalid"; message: string };

export interface AppInfo {
  version: string;
  build: string;
  platform: string;
}

export type AppDestination =
  | "Overview"
  | "Processes"
  | "Incidents"
  | "Connections"
  | "Settings"
  | "ProcessDefinition"
  | "ProcessInstance";

export interface DestinationMeta {
  id: AppDestination;
  title: string;
  description: string;
  icon: string;
  shortcut: string;
}

export const SIDEBAR_DESTINATIONS: DestinationMeta[] = [
  { id: "Overview", title: "Монитор", description: "Что требует внимания прямо сейчас", icon: "monitor", shortcut: "1" },
  { id: "Processes", title: "Поиск", description: "Заявки по бизнес-ключу и переменным", icon: "search", shortcut: "2" },
  { id: "Incidents", title: "Инциденты", description: "Группы ошибок и массовый повтор", icon: "alert", shortcut: "3" },
  { id: "Connections", title: "Подключения", description: "Окружения Camunda", icon: "plug", shortcut: "4" },
  { id: "Settings", title: "Настройки", description: "Тема и обновления", icon: "settings", shortcut: "5" },
];
