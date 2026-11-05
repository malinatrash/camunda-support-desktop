import { api } from "../api";
import type { AppThemeMode } from "../types";

export const THEME_LABEL: Record<AppThemeMode, string> = { Light: "Светлая", Dark: "Тёмная" };
export const FONT_SIZES = [12, 13, 14, 15, 16] as const;
export const DEFAULT_FONT_SIZE = 14;

const FONT_SIZE_KEY = "font-size";

class ThemeStore {
  mode = $state<AppThemeMode>("Light");
  fontSize = $state(DEFAULT_FONT_SIZE);

  async load() {
    this.applyFontSize(readFontSize());
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

  setFontSize(size: number) {
    this.applyFontSize(size);
    try {
      localStorage.setItem(FONT_SIZE_KEY, String(size));
    } catch {
      // Размер уже применён; без хранилища он сбросится при перезапуске.
    }
  }

  private applyFontSize(size: number) {
    this.fontSize = size;
    document.documentElement.style.setProperty("--font-size", `${size}px`);
  }

  private apply(mode: AppThemeMode) {
    this.mode = mode;
    document.documentElement.dataset.theme = mode === "Dark" ? "dark" : "light";
  }
}

export const theme = new ThemeStore();

function readFontSize(): number {
  try {
    const stored = Number(localStorage.getItem(FONT_SIZE_KEY));
    return (FONT_SIZES as readonly number[]).includes(stored) ? stored : DEFAULT_FONT_SIZE;
  } catch {
    return DEFAULT_FONT_SIZE;
  }
}
