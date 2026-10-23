<script lang="ts">
  import { onMount } from "svelte";
  import Button from "../components/Button.svelte";
  import SectionTitle from "../components/SectionTitle.svelte";
  import Spinner from "../components/Spinner.svelte";
  import AnimatedNumber from "../components/AnimatedNumber.svelte";
  import { theme, THEME_LABEL } from "../state/theme.svelte";
  import { updates } from "../state/updates.svelte";
  import { api, errorMessage } from "../api";
  import { grouped } from "../format";
  import { rise } from "../motion";
  import type { AppThemeMode, ReleaseDownloadStats } from "../types";

  let stats = $state<ReleaseDownloadStats | null>(null);
  let statsError = $state<string | null>(null);
  let statsLoading = $state(false);

  async function loadStats(force: boolean) {
    statsLoading = true;
    try {
      stats = await api.loadReleaseDownloadStats(force);
      statsError = null;
    } catch (error) {
      statsError = errorMessage(error);
    } finally {
      statsLoading = false;
    }
  }

  onMount(() => void loadStats(false));

  const themes: AppThemeMode[] = ["Light", "Dark"];
</script>

<div class="page scroll">
  <SectionTitle title="Настройки" description="Оформление и локальные параметры приложения." />

  <section class="card block" in:rise>
    <h2 class="block-title">Цветовая тема</h2>
    <p class="muted small">Выбор сохраняется на этом компьютере и применяется сразу. По умолчанию используется светлая тема.</p>
    <div class="themes">
      {#each themes as mode (mode)}
        <button type="button" class="theme-option" class:selected={theme.mode === mode} onclick={() => theme.set(mode)}>
          <div class="preview preview-{mode.toLowerCase()}">
            <div class="preview-sidebar"></div>
            <div class="preview-main">
              <div class="preview-line"></div>
              <div class="preview-line accent"></div>
              <div class="preview-block"></div>
            </div>
          </div>
          <div class="row theme-label">
            <span class="radio" class:on={theme.mode === mode}></span>
            <span class="col">
              <span class="theme-name">{THEME_LABEL[mode]}</span>
              <span class="faint">{mode === "Light" ? "Светлый фон" : "Тёмный фон"}</span>
            </span>
          </div>
        </button>
      {/each}
    </div>
  </section>

  <section class="card block" in:rise={{ delay: 60 }}>
    <div class="row">
      <div class="col grow">
        <h2 class="block-title">О приложении</h2>
        <p class="muted small">Версия {updates.info.version} · сборка {updates.info.build} · Tauri + Svelte</p>
      </div>
      <Button loading={statsLoading} onclick={() => loadStats(true)}>{statsLoading ? "Обновляем…" : "Обновить статистику"}</Button>
    </div>
    {#if updates.lastCheckError}
      <p class="small warning">Проверка обновлений: {updates.lastCheckError}</p>
    {/if}
    <div class="stats">
      {#if stats}
        <div class="total">
          <span class="eyebrow">Всего скачиваний</span>
          <span class="total-value"><AnimatedNumber value={stats.totalDownloads} format={grouped} duration={800} /></span>
          <span class="muted small">Установщики всех опубликованных версий</span>
        </div>
        <div class="platforms">
          <div class="platform"><span class="muted small">macOS</span><span class="platform-value"><AnimatedNumber value={stats.macOsDownloads} format={grouped} /></span></div>
          <div class="platform"><span class="muted small">Windows</span><span class="platform-value"><AnimatedNumber value={stats.windowsDownloads} format={grouped} /></span></div>
          <div class="platform"><span class="muted small">Linux</span><span class="platform-value"><AnimatedNumber value={stats.linuxDownloads} format={grouped} /></span></div>
        </div>
        <p class="faint small">
          Учтено релизов: {stats.releaseCount} · установочных файлов: {stats.installerAssetCount}. Повторные скачивания также входят в счётчик.
        </p>
      {:else if statsError}
        <p class="small danger selectable">{statsError}</p>
        <p class="muted small">Проверьте доступ к api.github.com и повторите загрузку.</p>
      {:else}
        <Spinner size={16} label="Считаем скачивания всех релизов GitHub…" />
      {/if}
    </div>
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    height: 100%;
    padding-right: 4px;
  }
  .page > :global(*) {
    flex-shrink: 0;
  }
  .block {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .block-title {
    font-size: 14px;
    font-weight: 650;
  }
  .small {
    font-size: 11px;
  }
  .warning {
    color: var(--warning);
  }
  .danger {
    color: var(--danger);
  }
  .themes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .theme-option {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--surface);
    text-align: left;
    transition: border-color var(--dur-fast) var(--ease-out), box-shadow var(--dur-base) var(--ease-out), transform var(--dur-base) var(--ease-spring);
  }
  .theme-option:hover {
    box-shadow: var(--shadow-2);
  }
  .theme-option.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .preview {
    height: 76px;
    border-radius: 8px;
    padding: 9px;
    display: flex;
    gap: 7px;
  }
  .preview-light { background: #f3f5f4; }
  .preview-dark { background: #0e1114; }
  .preview-sidebar { width: 32px; border-radius: 5px; }
  .preview-light .preview-sidebar { background: #fff; }
  .preview-dark .preview-sidebar { background: #13171c; }
  .preview-main { flex: 1; display: flex; flex-direction: column; gap: 6px; }
  .preview-line { height: 12px; border-radius: 4px; }
  .preview-light .preview-line { background: #fff; }
  .preview-dark .preview-line { background: #191e24; }
  .preview-line.accent { width: 62%; height: 8px; background: #16a34a !important; }
  .preview-dark .preview-line.accent { background: #3ddc84 !important; }
  .preview-block { flex: 1; border-radius: 4px; }
  .preview-light .preview-block { background: #f7f9f8; }
  .preview-dark .preview-block { background: #1f252c; }
  .theme-label {
    gap: 10px;
  }
  .radio {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--border-strong);
    display: grid;
    place-items: center;
    transition: border-color var(--dur-fast);
  }
  .radio::after {
    content: "";
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    transform: scale(0);
    transition: transform var(--dur-base) var(--ease-spring);
  }
  .radio.on {
    border-color: var(--accent);
  }
  .radio.on::after {
    transform: scale(1);
  }
  .theme-name {
    font-weight: 600;
    font-size: 12.5px;
  }
  .stats {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .total {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 16px;
    border-radius: 10px;
    background: var(--surface-2);
  }
  .total-value {
    font-size: 30px;
    font-weight: 750;
    color: var(--accent);
    letter-spacing: -0.02em;
  }
  .platforms {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .platform {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px;
    border-radius: 9px;
    background: var(--surface-2);
  }
  .platform-value {
    font-size: 18px;
    font-weight: 700;
  }
</style>
