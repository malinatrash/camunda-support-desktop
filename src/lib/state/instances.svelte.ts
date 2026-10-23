import { api, errorMessage } from "../api";
import type { CamundaConnection, ProcessInstanceDetails } from "../types";

export const INSTANCE_POLLING_SECONDS = 20;
const MAX_CACHED_INSTANCES = 32;

const EMPTY_ENTRY: InstanceEntry = { details: null, isRefreshing: false, error: null, lastSuccessfulRefresh: null };

export interface InstanceEntry {
  details: ProcessInstanceDetails | null;
  isRefreshing: boolean;
  error: string | null;
  lastSuccessfulRefresh: number | null;
}

// Общий кэш открытых заявок: переключение вкладок не показывает пустой экран.
export class InstanceStore {
  entries = $state<Record<string, InstanceEntry>>({});
  private order: string[] = [];

  key(connection: CamundaConnection, processInstanceId: string): string {
    return `${connection.id}|${processInstanceId}`;
  }

  // Чтение без мутации: подходит для $derived; запись в кэш делает только refresh.
  entry(connection: CamundaConnection, processInstanceId: string): InstanceEntry {
    return this.entries[this.key(connection, processInstanceId)] ?? EMPTY_ENTRY;
  }

  private ensure(connection: CamundaConnection, processInstanceId: string): InstanceEntry {
    const key = this.key(connection, processInstanceId);
    if (!this.entries[key]) {
      if (this.order.length >= MAX_CACHED_INSTANCES) {
        const evicted = this.order.shift();
        if (evicted) delete this.entries[evicted];
      }
      this.order.push(key);
      this.entries[key] = { details: null, isRefreshing: false, error: null, lastSuccessfulRefresh: null };
    }
    return this.entries[key];
  }

  async refresh(connection: CamundaConnection, processInstanceId: string, force = false): Promise<InstanceEntry> {
    const entry = this.ensure(connection, processInstanceId);
    const fresh =
      entry.lastSuccessfulRefresh != null && Date.now() - entry.lastSuccessfulRefresh < INSTANCE_POLLING_SECONDS * 1000;
    if (entry.isRefreshing || (!force && fresh)) return entry;
    entry.isRefreshing = true;
    try {
      entry.details = await api.loadProcessInstanceDetails(connection, processInstanceId);
      entry.error = null;
      entry.lastSuccessfulRefresh = Date.now();
    } catch (error) {
      entry.error = errorMessage(error) || "Не удалось обновить состояние заявки";
    } finally {
      entry.isRefreshing = false;
    }
    return entry;
  }
}

export const instanceStore = new InstanceStore();
