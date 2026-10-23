import { api } from "../api";
import { recents } from "./recents.svelte";
import { LIVE_FILTER } from "../types";
import type {
  AppDestination,
  CamundaConnection,
  ConnectionDraft,
  DashboardDateFilter,
  ProcessInstanceDeepLink,
  ProcessInstanceSummary,
} from "../types";

export interface OpenProcessTab {
  id: string;
  connectionId: string;
  connectionName: string;
  processInstanceId: string;
  businessKey: string | null;
  processDefinitionKey: string | null;
  returnDestination: AppDestination;
}

export type DeepLinkNavigationResult =
  | { kind: "opened" }
  | { kind: "connectionNotFound"; link: ProcessInstanceDeepLink }
  | { kind: "choiceRequired"; link: ProcessInstanceDeepLink; connections: CamundaConnection[] };

const MAX_OPEN_PROCESS_TABS = 12;

function normalizedOrigin(value: string): string | null {
  const trimmed = value.trim();
  const separator = trimmed.indexOf("://");
  if (separator <= 0) return null;
  const scheme = trimmed.slice(0, separator).toLowerCase();
  if (scheme !== "http" && scheme !== "https") return null;
  const rest = trimmed.slice(separator + 3);
  const end = rest.search(/[/?#]/);
  let authority = (end === -1 ? rest : rest.slice(0, end)).toLowerCase();
  if (!authority || /\s/.test(authority) || authority.includes("@")) return null;
  if (scheme === "http" && authority.endsWith(":80")) authority = authority.slice(0, -3);
  if (scheme === "https" && authority.endsWith(":443")) authority = authority.slice(0, -4);
  if (!authority) return null;
  return `${scheme}://${authority}`;
}

export function connectionMatchesOrigin(connection: CamundaConnection, origin: string): boolean {
  const normalized = normalizedOrigin(origin);
  if (!normalized) return false;
  return normalizedOrigin(connection.cockpitUrl) === normalized || normalizedOrigin(connection.restUrl) === normalized;
}

// Главное состояние приложения: подключения, навигация, вкладки открытых заявок.
export class AppStore {
  connections = $state<CamundaConnection[]>([]);
  openProcessTabs = $state<OpenProcessTab[]>([]);
  activeProcessTabId = $state<string | null>(null);
  selectedConnectionId = $state<string | null>(null);
  destination = $state<AppDestination>("Overview");
  isConnectionDialogOpen = $state(false);
  selectedProcessDefinitionId = $state<string | null>(null);
  selectedProcessInstanceId = $state<string | null>(null);
  selectedDashboardDateFilter = $state<DashboardDateFilter>(LIVE_FILTER);
  selectedIncidentProcessDefinitionId = $state<string | null>(null);
  selectedIncidentDateFilter = $state<DashboardDateFilter>(LIVE_FILTER);
  ready = $state(false);
  navigationSerial = $state(0);

  private processInstanceReturnDestination: AppDestination = "Processes";

  selectedConnection = $derived(this.connections.find((connection) => connection.id === this.selectedConnectionId) ?? null);

  async load() {
    const connections = await api.listConnections();
    this.connections = connections;
    this.selectedConnectionId = connections[0]?.id ?? null;
    this.ready = true;
  }

  selectConnection(connectionId: string) {
    if (!this.connections.some((connection) => connection.id === connectionId)) return;
    this.selectedConnectionId = connectionId;
    this.selectedProcessDefinitionId = null;
    this.selectedProcessInstanceId = null;
    this.activeProcessTabId = null;
    this.go("Overview");
  }

  navigate(destination: AppDestination) {
    if (destination === "Incidents") {
      this.selectedIncidentProcessDefinitionId = null;
      this.selectedIncidentDateFilter = LIVE_FILTER;
    }
    if (destination !== "ProcessInstance") {
      this.selectedProcessInstanceId = null;
      this.activeProcessTabId = null;
    }
    this.go(destination);
  }

  openIncidents(processDefinitionId: string | null, dateFilter: DashboardDateFilter) {
    this.selectedIncidentProcessDefinitionId = processDefinitionId;
    this.selectedIncidentDateFilter = dateFilter;
    this.activeProcessTabId = null;
    this.go("Incidents");
  }

  openProcessDefinition(processDefinitionId: string, dateFilter: DashboardDateFilter = LIVE_FILTER) {
    this.selectedProcessDefinitionId = processDefinitionId;
    this.selectedDashboardDateFilter = dateFilter;
    this.selectedProcessInstanceId = null;
    this.activeProcessTabId = null;
    this.go("ProcessDefinition");
  }

  openProcessInstance(processInstanceId: string, connectionId?: string) {
    if (connectionId && connectionId !== this.selectedConnectionId) this.selectConnection(connectionId);
    const connection = this.selectedConnection;
    if (!connection) return;
    if (this.destination !== "ProcessInstance") this.processInstanceReturnDestination = this.destination;
    const tabId = processTabId(connection.id, processInstanceId);
    if (!this.openProcessTabs.some((tab) => tab.id === tabId)) {
      if (this.openProcessTabs.length >= MAX_OPEN_PROCESS_TABS) this.openProcessTabs.shift();
      this.openProcessTabs.push({
        id: tabId,
        connectionId: connection.id,
        connectionName: connection.name,
        processInstanceId,
        businessKey: null,
        processDefinitionKey: null,
        returnDestination: this.processInstanceReturnDestination,
      });
    }
    this.activeProcessTabId = tabId;
    this.selectedProcessInstanceId = processInstanceId;
    recents.touch({
      connectionId: connection.id,
      connectionName: connection.name,
      processInstanceId,
      businessKey: null,
      processDefinitionKey: null,
    });
    this.go("ProcessInstance");
  }

  activateProcessTab(tabId: string) {
    const tab = this.openProcessTabs.find((candidate) => candidate.id === tabId);
    if (!tab) return;
    if (!this.connections.some((connection) => connection.id === tab.connectionId)) {
      this.openProcessTabs = this.openProcessTabs.filter((candidate) => candidate.id !== tabId);
      return;
    }
    this.selectedConnectionId = tab.connectionId;
    this.selectedProcessDefinitionId = null;
    this.selectedProcessInstanceId = tab.processInstanceId;
    this.processInstanceReturnDestination = tab.returnDestination;
    this.activeProcessTabId = tab.id;
    this.go("ProcessInstance");
  }

  closeProcessTab(tabId: string) {
    const index = this.openProcessTabs.findIndex((tab) => tab.id === tabId);
    if (index < 0) return;
    const wasActive = this.activeProcessTabId === tabId;
    this.openProcessTabs.splice(index, 1);
    if (!wasActive) return;
    const replacement = this.openProcessTabs[Math.min(index, this.openProcessTabs.length - 1)];
    if (replacement) {
      this.activateProcessTab(replacement.id);
    } else {
      this.activeProcessTabId = null;
      this.selectedProcessInstanceId = null;
      this.go("Overview");
    }
  }

  updateOpenProcessTab(instance: ProcessInstanceSummary) {
    const connectionId = this.selectedConnectionId;
    if (!connectionId) return;
    const tab = this.openProcessTabs.find((candidate) => candidate.id === processTabId(connectionId, instance.id));
    if (tab) {
      tab.businessKey = instance.businessKey;
      tab.processDefinitionKey = instance.definitionKey;
    }
    recents.update(connectionId, instance.id, { businessKey: instance.businessKey, processDefinitionKey: instance.definitionKey });
  }

  openDeepLink(link: ProcessInstanceDeepLink): DeepLinkNavigationResult {
    const matching = this.connections.filter((connection) => connectionMatchesOrigin(connection, link.origin));
    if (matching.length === 0) return { kind: "connectionNotFound", link };
    const target =
      matching.length === 1 ? matching[0] : (matching.find((connection) => connection.id === this.selectedConnectionId) ?? null);
    if (!target) return { kind: "choiceRequired", link, connections: matching };
    this.openDeepLinkWithConnection(target.id, link);
    return { kind: "opened" };
  }

  openDeepLinkWithConnection(connectionId: string, link: ProcessInstanceDeepLink) {
    const connection = this.connections.find(
      (candidate) => candidate.id === connectionId && connectionMatchesOrigin(candidate, link.origin),
    );
    if (!connection) return;
    this.selectConnection(connection.id);
    this.openProcessInstance(link.processInstanceId);
  }

  backFromProcessDefinition() {
    this.activeProcessTabId = null;
    this.go("Overview");
  }

  backFromProcessInstance() {
    const fallback =
      this.processInstanceReturnDestination === "ProcessDefinition" && this.selectedProcessDefinitionId == null
        ? "Overview"
        : this.processInstanceReturnDestination;
    this.selectedProcessInstanceId = null;
    this.activeProcessTabId = null;
    this.go(fallback);
  }

  openConnectionDialog() {
    this.isConnectionDialogOpen = true;
  }

  closeConnectionDialog() {
    this.isConnectionDialogOpen = false;
  }

  async addConnection(draft: ConnectionDraft, engineVersion: string | null) {
    const connection = await api.addConnection(draft, engineVersion);
    this.connections = [...this.connections, connection].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    this.selectedConnectionId = connection.id;
    this.isConnectionDialogOpen = false;
    this.go("Overview");
  }

  async deleteConnection(connectionId: string) {
    await api.deleteConnection(connectionId);
    this.connections = this.connections.filter((connection) => connection.id !== connectionId);
    this.openProcessTabs = this.openProcessTabs.filter((tab) => tab.connectionId !== connectionId);
    recents.forgetConnection(connectionId);
    if (this.selectedConnectionId === connectionId) {
      this.selectedConnectionId = this.connections[0]?.id ?? null;
    }
  }

  private go(destination: AppDestination) {
    this.destination = destination;
    this.navigationSerial += 1;
  }
}

function processTabId(connectionId: string, processInstanceId: string): string {
  return `${connectionId}::${processInstanceId}`;
}

export const app = new AppStore();
