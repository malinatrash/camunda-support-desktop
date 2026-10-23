import type { CamundaConnection, ConnectionDraft, ConnectionValidation } from "./types";

function genericUrlError(value: string, required: boolean): string | null {
  if (value.trim() === "") return required ? "Укажите URL" : null;
  const normalized = value.trim();
  if (!normalized.startsWith("https://") && !normalized.startsWith("http://")) {
    return "URL должен начинаться с https:// или http://";
  }
  if (/\s/.test(normalized)) return "URL не должен содержать пробелы";
  if (normalized.includes("?") || normalized.includes("#")) return "URL не должен содержать query-параметры или fragment";
  const authorityAndPath = normalized.slice(normalized.indexOf("://") + 3);
  if ((authorityAndPath.split("/")[0] ?? "").trim() === "") return "URL должен содержать адрес хоста";
  return null;
}

export function validateConnectionDraft(draft: ConnectionDraft): ConnectionValidation {
  let restUrlError = genericUrlError(draft.restUrl, true);
  if (!restUrlError && !draft.restUrl.trim().replace(/\/+$/, "").endsWith("/engine-rest")) {
    restUrlError = "Адрес Camunda REST должен заканчиваться на /engine-rest";
  }
  return {
    nameError: draft.name.trim() === "" ? "Укажите название" : null,
    restUrlError,
    cockpitUrlError: genericUrlError(draft.cockpitUrl, false),
  };
}

export function isValidConnection(validation: ConnectionValidation): boolean {
  return !validation.nameError && !validation.restUrlError && !validation.cockpitUrlError;
}

export function deriveCockpitUrl(restUrl: string): string {
  const normalized = restUrl.trim();
  const separator = normalized.indexOf("://");
  if (separator <= 0) return "";
  const authorityStart = separator + 3;
  const slash = normalized.indexOf("/", authorityStart);
  const authorityEnd = slash === -1 ? normalized.length : slash;
  if (authorityEnd <= authorityStart) return "";
  return `${normalized.slice(0, authorityEnd)}/camunda/app/cockpit`;
}

function cockpitRoute(connection: CamundaConnection, route: string): string | null {
  const base = connection.cockpitUrl.trim().replace(/\/+$/, "");
  return base === "" ? null : `${base}/default/#/${route}`;
}

export function processDefinitionCockpitUrl(connection: CamundaConnection, processDefinitionId: string): string | null {
  return cockpitRoute(connection, `process-definition/${processDefinitionId}`);
}

export function processInstanceCockpitUrl(connection: CamundaConnection, processInstanceId: string): string | null {
  return cockpitRoute(connection, `process-instance/${processInstanceId}/runtime`);
}

export function cockpitDashboardUrl(connection: CamundaConnection): string | null {
  return cockpitRoute(connection, "dashboard");
}
