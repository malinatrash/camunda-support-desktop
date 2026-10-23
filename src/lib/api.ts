import { Channel, invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type {
  AppInfo,
  AppThemeMode,
  AppUpdate,
  BulkRetryResult,
  CamundaConnection,
  ConnectionDraft,
  ConnectionTestSuccess,
  ConnectionValidation,
  DashboardDateFilter,
  DeepLinkEvent,
  ProcessDashboard,
  ProcessDefinitionDetails,
  ProcessDefinitionSummary,
  ProcessIncident,
  ProcessInstanceDetails,
  ProcessInstanceSummary,
  ProcessVariableCatalog,
  ProcessVariableUpdate,
  ReleaseDownloadStats,
  TeleportRequest,
  UpdateCheckResult,
  UpdateDownloadProgress,
  UpdateInstallResult,
  VariableValueType,
} from "./types";

// Любая ошибка команды Tauri приходит строкой на русском; приводим всё остальное к строке.
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error ?? "Неизвестная ошибка");
}

const tauriApi = {
  appInfo: () => invoke<AppInfo>("app_info"),

  listConnections: () => invoke<CamundaConnection[]>("list_connections"),
  addConnection: (draft: ConnectionDraft, engineVersion: string | null) =>
    invoke<CamundaConnection>("add_connection", { draft, engineVersion }),
  deleteConnection: (connectionId: string) => invoke<void>("delete_connection", { connectionId }),
  validateConnectionDraft: (draft: ConnectionDraft) =>
    invoke<ConnectionValidation>("validate_connection_draft", { draft }),
  deriveCockpitUrl: (restUrl: string) => invoke<string>("derive_cockpit_url", { restUrl }),
  testConnection: (restUrl: string) => invoke<ConnectionTestSuccess>("test_connection", { restUrl }),

  getTheme: () => invoke<AppThemeMode>("get_theme"),
  setTheme: (theme: AppThemeMode) => invoke<void>("set_theme", { theme }),

  loadDashboard: (connection: CamundaConnection, dateFilter: DashboardDateFilter) =>
    invoke<ProcessDashboard>("load_dashboard", { connection, dateFilter }),
  loadIncidents: (
    connection: CamundaConnection,
    processDefinitionId: string | null,
    dateFilter: DashboardDateFilter,
    processDefinitionKey: string | null,
  ) =>
    invoke<ProcessIncident[]>("load_incidents", {
      connection,
      processDefinitionId,
      dateFilter,
      processDefinitionKey,
    }),
  loadProcessDefinitionVersions: (connection: CamundaConnection, processDefinitionKey: string) =>
    invoke<ProcessDefinitionSummary[]>("load_process_definition_versions", { connection, processDefinitionKey }),
  searchProcessInstances: (
    connection: CamundaConnection,
    processDefinitionKey: string,
    variableName: string,
    variableValue: string,
    valueType: VariableValueType,
  ) =>
    invoke<ProcessInstanceSummary[]>("search_process_instances", {
      connection,
      processDefinitionKey,
      variableName,
      variableValue,
      valueType,
    }),
  loadSearchProcessDefinitions: (connection: CamundaConnection) =>
    invoke<ProcessDefinitionSummary[]>("load_search_process_definitions", { connection }),
  loadProcessVariableCatalog: (connection: CamundaConnection, processDefinitionKey: string) =>
    invoke<ProcessVariableCatalog>("load_process_variable_catalog", { connection, processDefinitionKey }),
  invalidateProcessVariableCatalog: (connection: CamundaConnection, processDefinitionKey: string) =>
    invoke<void>("invalidate_process_variable_catalog", { connection, processDefinitionKey }),
  loadProcessDefinitionDetails: (
    connection: CamundaConnection,
    processDefinitionId: string,
    dateFilter: DashboardDateFilter,
  ) => invoke<ProcessDefinitionDetails>("load_process_definition_details", { connection, processDefinitionId, dateFilter }),
  loadProcessInstanceDetails: (connection: CamundaConnection, processInstanceId: string) =>
    invoke<ProcessInstanceDetails>("load_process_instance_details", { connection, processInstanceId }),
  searchInstancesByBusinessKey: (connection: CamundaConnection, query: string) =>
    invoke<ProcessInstanceSummary[]>("search_instances_by_business_key", { connection, query }),
  retryIncidents: (connection: CamundaConnection, incidents: ProcessIncident[], retries: number) =>
    invoke<BulkRetryResult>("retry_incidents", { connection, incidents, retries }),
  teleportProcessInstance: (connection: CamundaConnection, processInstanceId: string, request: TeleportRequest) =>
    invoke<void>("teleport_process_instance", { connection, processInstanceId, request }),
  setJobRetries: (connection: CamundaConnection, jobId: string, retries: number) =>
    invoke<void>("set_job_retries", { connection, jobId, retries }),
  setExternalTaskRetries: (connection: CamundaConnection, externalTaskId: string, retries: number) =>
    invoke<void>("set_external_task_retries", { connection, externalTaskId, retries }),
  unlockExternalTask: (connection: CamundaConnection, externalTaskId: string) =>
    invoke<void>("unlock_external_task", { connection, externalTaskId }),
  setProcessInstanceSuspended: (connection: CamundaConnection, processInstanceId: string, suspended: boolean) =>
    invoke<void>("set_process_instance_suspended", { connection, processInstanceId, suspended }),
  updateProcessVariable: (connection: CamundaConnection, processInstanceId: string, update: ProcessVariableUpdate) =>
    invoke<void>("update_process_variable", { connection, processInstanceId, update }),

  checkForUpdate: () => invoke<UpdateCheckResult>("check_for_update"),
  downloadAndOpenUpdate: (update: AppUpdate, onProgress: (progress: UpdateDownloadProgress) => void) => {
    const channel = new Channel<UpdateDownloadProgress>();
    channel.onmessage = onProgress;
    return invoke<UpdateInstallResult>("download_and_open_update", { update, onProgress: channel });
  },
  loadReleaseDownloadStats: (forceRefresh: boolean) =>
    invoke<ReleaseDownloadStats>("load_release_download_stats", { forceRefresh }),

  parseDeepLink: (raw: string) => invoke<DeepLinkEvent>("parse_deep_link", { raw }),
  processInstanceDeepLink: (connection: CamundaConnection, processInstanceId: string) =>
    invoke<string | null>("process_instance_deep_link", { connection, processInstanceId }),
  takePendingDeepLinks: () => invoke<DeepLinkEvent[]>("take_pending_deep_links"),
  openInBrowser: (url: string) => invoke<void>("open_in_browser", { url }),
};

// В обычном браузере (pnpm dev без Tauri) интерфейс работает на демо-данных.
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
// Копирует текст в системный буфер; в браузере — через navigator.clipboard.
export async function copyText(text: string): Promise<boolean> {
  try {
    if (inTauri) await writeText(text);
    else await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

export const api: typeof tauriApi = inTauri ? tauriApi : (await import("./mock/mock")).mockApi;
