import { untrack } from "svelte";
import { api, errorMessage } from "../api";
import { dateFilterKey } from "../format";
import type { CamundaConnection, DashboardDateFilter, ProcessDashboard } from "../types";

export const DASHBOARD_POLLING_SECONDS = 15;

export interface DashboardEntry {
  dashboard: ProcessDashboard | null;
  isRefreshing: boolean;
  error: string | null;
  lastSuccessfulRefresh: number | null;
}

const EMPTY_ENTRY: DashboardEntry = { dashboard: null, isRefreshing: false, error: null, lastSuccessfulRefresh: null };

function queryKey(connection: CamundaConnection, dateFilter: DashboardDateFilter): string {
  return `${connection.id}|${dateFilterKey(dateFilter)}`;
}

// Кэш дашбордов по подключению и периоду с защитой от параллельных обновлений.
export class DashboardStore {
  entries = $state<Record<string, DashboardEntry>>({});

  // Чтение без мутации: подходит для $derived; запись в кэш делает только refresh.
  entry(connection: CamundaConnection, dateFilter: DashboardDateFilter): DashboardEntry {
    return this.entries[queryKey(connection, dateFilter)] ?? EMPTY_ENTRY;
  }

  // untrack: вызов из $effect не должен подписывать эффект на поля записи, иначе каждый ответ перезапускает загрузку.
  refresh(connection: CamundaConnection, dateFilter: DashboardDateFilter, force = false): Promise<void> {
    return untrack(() => this.load(connection, dateFilter, force));
  }

  private async load(connection: CamundaConnection, dateFilter: DashboardDateFilter, force: boolean) {
    const key = queryKey(connection, dateFilter);
    this.entries[key] ??= { ...EMPTY_ENTRY };
    const entry = this.entries[key];
    const fresh =
      entry.lastSuccessfulRefresh != null && Date.now() - entry.lastSuccessfulRefresh < DASHBOARD_POLLING_SECONDS * 1000;
    if (entry.isRefreshing || (!force && fresh)) return;
    entry.isRefreshing = true;
    try {
      entry.dashboard = await api.loadDashboard(connection, dateFilter);
      entry.error = null;
      entry.lastSuccessfulRefresh = Date.now();
    } catch (error) {
      entry.error = errorMessage(error) || "Не удалось обновить дашборд";
    } finally {
      entry.isRefreshing = false;
    }
  }
}

export const dashboardStore = new DashboardStore();
