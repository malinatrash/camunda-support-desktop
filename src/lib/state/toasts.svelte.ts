export type ToastTone = "info" | "success" | "danger";

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
}

class ToastStore {
  items = $state<Toast[]>([]);
  private serial = 0;

  show(message: string, tone: ToastTone = "info", durationMs = 3200) {
    const id = ++this.serial;
    this.items.push({ id, message, tone });
    setTimeout(() => this.dismiss(id), durationMs);
  }

  success(message: string) {
    this.show(message, "success");
  }

  error(message: string) {
    this.show(message, "danger", 5000);
  }

  dismiss(id: number) {
    this.items = this.items.filter((toast) => toast.id !== id);
  }
}

export const toasts = new ToastStore();
