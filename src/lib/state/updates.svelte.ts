import { api, errorMessage } from "../api";
import type { AppInfo, AppUpdate, UpdateDownloadProgress } from "../types";

const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

// Проверка новых релизов GitHub при запуске и каждые шесть часов, загрузка установщика с прогрессом.
class UpdateStore {
  info = $state<AppInfo>({ version: "0.0.0", build: "local", platform: "" });
  available = $state<AppUpdate | null>(null);
  dismissedTag = $state<string | null>(null);
  downloading = $state(false);
  progress = $state<UpdateDownloadProgress | null>(null);
  error = $state<string | null>(null);
  openedInstaller = $state<string | null>(null);
  lastCheckError = $state<string | null>(null);
  private timer: ReturnType<typeof setInterval> | null = null;

  visible = $derived(this.available != null && this.available.tag !== this.dismissedTag ? this.available : null);

  async start() {
    try {
      this.info = await api.appInfo();
    } catch {
      // Версию покажем как local.
    }
    await this.check();
    this.timer = setInterval(() => void this.check(), UPDATE_CHECK_INTERVAL_MS);
  }

  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  }

  async check() {
    try {
      const result = await api.checkForUpdate();
      if (result.kind === "available") {
        if (this.available?.tag !== result.update.tag) {
          this.progress = null;
          this.error = null;
          this.openedInstaller = null;
        }
        this.available = result.update;
        this.lastCheckError = null;
      } else if (result.kind === "upToDate") {
        this.available = null;
        this.lastCheckError = null;
      } else {
        this.lastCheckError = result.message;
      }
    } catch (error) {
      this.lastCheckError = errorMessage(error);
    }
  }

  async install() {
    const update = this.available;
    if (!update) return;
    if (!update.asset) {
      await api.openInBrowser(update.releaseUrl);
      return;
    }
    this.downloading = true;
    this.error = null;
    this.openedInstaller = null;
    this.progress = null;
    try {
      const result = await api.downloadAndOpenUpdate(update, (progress) => {
        this.progress = progress;
      });
      if (result.kind === "installerOpened") this.openedInstaller = result.fileName;
      else this.error = result.message;
    } catch (error) {
      this.error = errorMessage(error);
    } finally {
      this.downloading = false;
    }
  }

  dismiss() {
    this.dismissedTag = this.available?.tag ?? null;
  }
}

export const updates = new UpdateStore();
