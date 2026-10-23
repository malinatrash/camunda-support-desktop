import { api, errorMessage } from "../api";
import { dateFilterKey } from "../format";
import type { CamundaConnection, DashboardDateFilter, ProcessDashboard } from "../types";

export const DASHBOARD_POLLING_SECONDS = 10;

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
  secondsUntilRefresh = $state(DASHBOARD_POLLING_SECONDS);

  // Чтение без мутации: подходит для $derived; запись в кэш делает только refresh.
  entry(connection: CamundaConnection, dateFilter: DashboardDateFilter): DashboardEntry {
    return this.entries[queryKey(connection, dateFilter)] ?? EMPTY_ENTRY;
  }

  private ensure(connection: CamundaConnection, dateFilter: DashboardDateFilter): DashboardEntry {
    const key = queryKey(connection, dateFilter);
    if (!this.entries[key]) {
      this.entries[key] = { dashboard: null, isRefreshing: false, error: null, lastSuccessfulRefresh: null };
    }
    return this.entries[key];
  }

  async refresh(connection: CamundaConnection, dateFilter: DashboardDateFilter, force = false) {
    const entry = this.ensure(connection, dateFilter);
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
