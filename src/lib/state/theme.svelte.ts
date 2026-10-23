import { api } from "../api";
import type { AppThemeMode } from "../types";

export const THEME_LABEL: Record<AppThemeMode, string> = { Light: "Светлая", Dark: "Тёмная" };

class ThemeStore {
  mode = $state<AppThemeMode>("Light");

  async load() {
    try {
      this.apply(await api.getTheme());
    } catch {
      this.apply("Light");
    }
  }

  async set(mode: AppThemeMode) {
    this.apply(mode);
    try {
      await api.setTheme(mode);
    } catch {
      // Тема уже применена в интерфейсе; ошибка сохранения не критична.
    }
  }

  private apply(mode: AppThemeMode) {
    this.mode = mode;
    document.documentElement.dataset.theme = mode === "Dark" ? "dark" : "light";
  }
}

export const theme = new ThemeStore();
