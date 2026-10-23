import type { ProcessIncident } from "./types";

export interface IncidentGroup {
  key: string;
  activityId: string | null;
  processDefinitionId: string | null;
  definitionKey: string | null;
  type: string;
  signature: string;
  sampleMessage: string | null;
  count: number;
  instanceIds: string[];
  incidents: ProcessIncident[];
  latestTimestamp: string | null;
}

// Сжимает сообщение ошибки в подпись: без идентификаторов и чисел, одна строка, до 140 символов.
export function incidentSignature(message: string | null | undefined): string {
  if (!message) return "Без сообщения";
  const firstLine = message.split(/\r?\n/).find((line) => line.trim() !== "") ?? message;
  return firstLine
    .replace(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi, "#")
    .replace(/\b[0-9a-f]{16,}\b/gi, "#")
    .replace(/\d+/g, "#")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 140);
}

// Группирует инциденты по кубику, процессу и подписи сообщения; группы отсортированы по размеру.
export function groupIncidents(incidents: ProcessIncident[]): IncidentGroup[] {
  const groups = new Map<string, IncidentGroup>();
  for (const incident of incidents) {
    const activityId = incident.failedActivityId ?? incident.activityId;
    const definitionKey = incident.processDefinitionId?.split(":")[0] ?? null;
    const signature = incidentSignature(incident.message);
    const key = `${definitionKey ?? ""}|${activityId ?? ""}|${incident.type}|${signature}`;
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        activityId,
        processDefinitionId: incident.processDefinitionId,
        definitionKey,
        type: incident.type,
        signature,
        sampleMessage: incident.message,
        count: 0,
        instanceIds: [],
        incidents: [],
        latestTimestamp: null,
      };
      groups.set(key, group);
    }
    group.count += 1;
    group.incidents.push(incident);
    if (incident.processInstanceId && !group.instanceIds.includes(incident.processInstanceId)) group.instanceIds.push(incident.processInstanceId);
    if (incident.timestamp && (!group.latestTimestamp || incident.timestamp > group.latestTimestamp)) group.latestTimestamp = incident.timestamp;
  }
  return [...groups.values()].sort((a, b) => b.count - a.count || (b.latestTimestamp ?? "").localeCompare(a.latestTimestamp ?? ""));
}

export function retryableCount(group: IncidentGroup): number {
  return group.incidents.filter((incident) => (incident.type === "failedJob" || incident.type === "failedExternalTask") && !!incident.configuration).length;
}

// Короткое «3 мин назад» для меток времени Camunda.
export function relativeTime(timestamp: string | null | undefined, now = Date.now()): string {
  if (!timestamp) return "—";
  const normalized = timestamp.replace(/([+-]\d{2})(\d{2})$/, "$1:$2");
  const value = Date.parse(normalized);
  if (Number.isNaN(value)) return timestamp;
  const seconds = Math.max(0, Math.round((now - value) / 1000));
  if (seconds < 60) return "только что";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} мин назад`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours} ч назад`;
  return `${Math.round(hours / 24)} д назад`;
}
