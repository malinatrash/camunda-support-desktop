import type { DashboardDateFilter, Environment, ProcessVariable, UpdateDownloadProgress, UpdateDownloadStage } from "./types";

export function formatBytes(value: number): string {
  if (!value || value <= 0) return "0 Б";
  const units = ["Б", "КБ", "МБ", "ГБ"];
  let amount = value;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  if (unit === 0) return `${value} ${units[unit]}`;
  const tenths = Math.floor(amount * 10);
  const decimal = tenths % 10;
  return decimal === 0 ? `${Math.floor(tenths / 10)} ${units[unit]}` : `${Math.floor(tenths / 10)},${decimal} ${units[unit]}`;
}

export function formatSeconds(seconds: number): string {
  if (seconds < 60) return `${seconds} с`;
  return `${Math.floor(seconds / 60)} мин ${seconds % 60} с`;
}

// Длительность в минутах/часах/днях для дашборда: «2 д. 4 ч.», «3 ч. 12 мин.», «7 мин.».
export function formatDuration(durationMillis: number | null | undefined): string {
  if (durationMillis == null) return "Нет данных";
  const minutes = Math.floor(durationMillis / 60_000);
  const days = Math.floor(minutes / (24 * 60));
  const hours = Math.floor((minutes % (24 * 60)) / 60);
  const restMinutes = minutes % 60;
  if (days > 0) return `${days} д. ${hours} ч.`;
  if (hours > 0) return `${hours} ч. ${restMinutes} мин.`;
  return `${restMinutes} мин.`;
}

export function grouped(value: number): string {
  const text = Math.trunc(value).toString();
  return text.replace(/\B(?=(\d{3})+(?!\d))/g, " ");
}

export function incidentTypeLabel(type: string): string {
  if (type === "failedJob") return "Ошибка задания";
  if (type === "failedExternalTask") return "Ошибка внешней задачи";
  return type.trim() === "" ? "Неизвестный тип" : type;
}

export function dashboardIncidentTypeLabel(type: string): string {
  if (type === "failedJob") return "Ошибка задачи";
  if (type === "failedExternalTask") return "Ошибка внешней задачи";
  return type.trim() === "" ? "Неизвестный тип" : type;
}

// «ключ · v3» из идентификатора определения вида key:3:uuid.
export function shortDefinitionLabel(definitionId: string): string {
  const parts = definitionId.split(":");
  const key = parts[0] ?? definitionId;
  const version = Number.parseInt(parts[1] ?? "", 10);
  return Number.isFinite(version) ? `${key} · v${version}` : key;
}

export function definitionKeyOf(definitionId: string): string | null {
  const key = definitionId.split(":")[0] ?? "";
  return key.trim() === "" ? null : key;
}

export function definitionVersionOf(definitionId: string): number | null {
  const version = Number.parseInt(definitionId.split(":")[1] ?? "", 10);
  return Number.isFinite(version) ? version : null;
}

export const ENVIRONMENT_LABEL: Record<Environment, { label: string; short: string }> = {
  Production: { label: "Прод", short: "ПРОД" },
  Test: { label: "Тест", short: "ТЕСТ" },
  Development: { label: "Разработка", short: "РАЗР" },
  Local: { label: "Локальное", short: "ЛОКАЛ" },
};

export const ENVIRONMENTS: Environment[] = ["Production", "Test", "Development", "Local"];

export function environmentTone(environment: Environment): "danger" | "warning" | "development" | "neutral" {
  switch (environment) {
    case "Production":
      return "danger";
    case "Test":
      return "warning";
    case "Development":
      return "development";
    default:
      return "neutral";
  }
}

export const DATE_PRESET_LABEL = { Live: "Сейчас", Today: "Сегодня", Yesterday: "Вчера", Custom: "Период" } as const;

export function isLive(filter: DashboardDateFilter): boolean {
  return filter.preset === "Live";
}

export function dateFilterKey(filter: DashboardDateFilter): string {
  return filter.preset === "Custom" ? `Custom:${filter.fromDate}:${filter.toDate}` : filter.preset;
}

export function instancesTitle(filter: DashboardDateFilter): string {
  switch (filter.preset) {
    case "Live":
      return "Заявки, запущенные сейчас";
    case "Today":
      return "Заявки, стартовавшие сегодня";
    case "Yesterday":
      return "Заявки, стартовавшие вчера";
    default:
      return `Заявки за период ${filter.fromDate} — ${filter.toDate}`;
  }
}

export const VALUE_TYPE_LABEL = { Auto: "Авто", String: "Строка", Number: "Число", Boolean: "Логическое" } as const;

export function updateStageLabel(stage: UpdateDownloadStage): string {
  switch (stage) {
    case "Preparing":
      return "Подготавливаем загрузку…";
    case "Downloading":
      return "Загружаем установщик…";
    case "Verifying":
      return "Проверяем SHA-256…";
    default:
      return "Открываем установщик…";
  }
}

export function progressFraction(progress: UpdateDownloadProgress): number | null {
  if (progress.totalBytes <= 0) return null;
  return Math.min(1, Math.max(0, progress.downloadedBytes / progress.totalBytes));
}

export function progressLabel(progress: UpdateDownloadProgress): string {
  const fraction = progressFraction(progress);
  const percentage = fraction == null ? "—" : `${Math.floor(fraction * 100)}%`;
  const remaining = Math.max(0, progress.totalBytes - progress.downloadedBytes);
  const eta = progress.bytesPerSecond > 0 ? formatSeconds(Math.max(0, Math.floor(remaining / progress.bytesPerSecond))) : null;
  if (progress.stage === "Downloading" && progress.totalBytes > 0) {
    return `${percentage} · осталось ${formatBytes(remaining)}${eta ? ` · ≈ ${eta}` : ""}`;
  }
  return updateStageLabel(progress.stage);
}

const READ_ONLY_TYPES = new Set(["object", "file", "bytes", "null"]);

export function isVariableEditable(variable: ProcessVariable): boolean {
  return !READ_ONLY_TYPES.has(variable.type.toLowerCase());
}

export function variableEditHint(variable: ProcessVariable): string {
  switch (variable.type.toLowerCase()) {
    case "boolean":
      return "Введите true или false.";
    case "short":
    case "integer":
    case "long":
      return "Введите целое число без пробелов и разделителей.";
    case "double":
      return "Введите число, дробная часть отделяется точкой.";
    case "date":
      return "Сохранится текущий тип Date; используйте формат даты Camunda с часовым поясом.";
    case "json":
      return "Введите полный корректный JSON. Он будет сохранён как строковое JSON-значение Camunda.";
    default:
      return `Тип переменной останется ${variable.type}.`;
  }
}

const INT_PATTERN = /^[+-]?\d+$/;

function inRange(text: string, min: bigint, max: bigint): boolean {
  if (!INT_PATTERN.test(text)) return false;
  const value = BigInt(text);
  return value >= min && value <= max;
}

export function validateVariableValue(variable: ProcessVariable, newValue: string): string | null {
  const value = newValue.trim();
  switch (variable.type.toLowerCase()) {
    case "boolean":
      return value.toLowerCase() === "true" || value.toLowerCase() === "false" ? null : "Допустимы только true или false.";
    case "short":
      return inRange(value, -32768n, 32767n) ? null : "Значение выходит за диапазон Short или не является целым числом.";
    case "integer":
      return inRange(value, -2147483648n, 2147483647n)
        ? null
        : "Значение выходит за диапазон Integer или не является целым числом.";
    case "long":
      return inRange(value, -9223372036854775808n, 9223372036854775807n)
        ? null
        : "Значение выходит за диапазон Long или не является целым числом.";
    case "double": {
      const number = Number(value);
      return value !== "" && Number.isFinite(number) ? null : "Введите конечное число в формате Double.";
    }
    default:
      return null;
  }
}

export function contains(haystack: string | null | undefined, query: string): boolean {
  if (!haystack) return false;
  return haystack.toLowerCase().includes(query.trim().toLowerCase());
}

export function pluralize(count: number, one: string, few: string, many: string): string {
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod10 === 1 && mod100 !== 11) return one;
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 10 || mod100 >= 20)) return few;
  return many;
}
