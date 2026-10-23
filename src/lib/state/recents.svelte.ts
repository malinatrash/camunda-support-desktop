export interface RecentInstance {
  connectionId: string;
  connectionName: string;
  processInstanceId: string;
  businessKey: string | null;
  processDefinitionKey: string | null;
  openedAt: number;
  pinned: boolean;
}

const STORAGE_KEY = "camunda-support.recents";
const LIMIT = 40;

function read(): RecentInstance[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as RecentInstance[]) : [];
  } catch {
    return [];
  }
}

// Недавние и закреплённые заявки — хранятся в браузерном хранилище webview.
class RecentsStore {
  items = $state<RecentInstance[]>(read());

  pinned = $derived(this.items.filter((item) => item.pinned).sort((a, b) => b.openedAt - a.openedAt));
  recent = $derived(this.items.filter((item) => !item.pinned).sort((a, b) => b.openedAt - a.openedAt).slice(0, 12));

  touch(entry: Omit<RecentInstance, "openedAt" | "pinned">) {
    const existing = this.items.find((item) => item.connectionId === entry.connectionId && item.processInstanceId === entry.processInstanceId);
    if (existing) {
      existing.openedAt = Date.now();
      existing.businessKey = entry.businessKey ?? existing.businessKey;
      existing.processDefinitionKey = entry.processDefinitionKey ?? existing.processDefinitionKey;
    } else {
      this.items.push({ ...entry, openedAt: Date.now(), pinned: false });
    }
    this.trim();
    this.persist();
  }

  update(connectionId: string, processInstanceId: string, patch: Partial<RecentInstance>) {
    const existing = this.items.find((item) => item.connectionId === connectionId && item.processInstanceId === processInstanceId);
    if (!existing) return;
    Object.assign(existing, patch);
    this.persist();
  }

  isPinned(connectionId: string, processInstanceId: string): boolean {
    return this.items.some((item) => item.connectionId === connectionId && item.processInstanceId === processInstanceId && item.pinned);
  }

  togglePin(connectionId: string, processInstanceId: string): boolean {
    const existing = this.items.find((item) => item.connectionId === connectionId && item.processInstanceId === processInstanceId);
    if (!existing) return false;
    existing.pinned = !existing.pinned;
    this.persist();
    return existing.pinned;
  }

  remove(connectionId: string, processInstanceId: string) {
    this.items = this.items.filter((item) => !(item.connectionId === connectionId && item.processInstanceId === processInstanceId));
    this.persist();
  }

  forgetConnection(connectionId: string) {
    this.items = this.items.filter((item) => item.connectionId !== connectionId);
    this.persist();
  }

  private trim() {
    const unpinned = this.items.filter((item) => !item.pinned).sort((a, b) => b.openedAt - a.openedAt);
    if (unpinned.length <= LIMIT) return;
    const drop = new Set(unpinned.slice(LIMIT).map((item) => `${item.connectionId}|${item.processInstanceId}`));
    this.items = this.items.filter((item) => !drop.has(`${item.connectionId}|${item.processInstanceId}`));
  }

  private persist() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.items));
    } catch {
      // Хранилище недоступно — список живёт только в памяти.
    }
  }
}

export const recents = new RecentsStore();
