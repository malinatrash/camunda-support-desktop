<script lang="ts">
  import Button from "./Button.svelte";
  import ProgressBar from "./ProgressBar.svelte";
  import { updates } from "../state/updates.svelte";
  import { api } from "../api";
  import { formatBytes, progressFraction, progressLabel, updateStageLabel } from "../format";

  const update = $derived(updates.visible);
  const progress = $derived(updates.progress);
  const status = $derived.by(() => {
    if (!update) return "";
    if (updates.openedInstaller) return `Установщик ${updates.openedInstaller} открыт. Завершите установку в системном окне.`;
    if (updates.error) return updates.error;
    if (!update.asset) return "Для этой платформы нет автоматического установщика. Откройте страницу релиза.";
    if (updates.downloading && progress) return updateStageLabel(progress.stage);
    return "Скачаем установщик с GitHub, проверим SHA-256 и откроем его.";
  });
  const showMeta = $derived(update?.asset != null && (updates.downloading || progress != null));
</script>

{#if update}
  <div class="banner">
    <div class="col grow">
      <span class="headline">Доступна новая версия {update.version}</span>
      <span class="status" class:error={updates.error != null}>{status}</span>
      {#if showMeta && update.asset}
        <div class="meta selectable mono">
          <span>Версия: {update.version} · Тег: {update.tag} · Файл: {update.asset.name}</span>
          <span>Размер: {formatBytes(update.asset.sizeBytes)} ({update.asset.sizeBytes} байт)</span>
          <span>SHA-256: {update.asset.digest ?? update.checksumUrl ?? "не указан"}</span>
          <span>Релиз: {update.releaseUrl}</span>
          {#if update.checksumUrl}<span>Контрольные суммы: {update.checksumUrl}</span>{/if}
          <span>Источник: {update.asset.downloadUrl}</span>
          {#if progress}<span>Сохранение: {progress.destinationPath}</span>{/if}
        </div>
      {/if}
      {#if updates.downloading && progress}
        <div class="progress">
          <ProgressBar fraction={progressFraction(progress)} />
          <div class="row progress-row">
            <span class="muted">
              {formatBytes(progress.downloadedBytes)} из {formatBytes(progress.totalBytes)}{progress.bytesPerSecond > 0
                ? ` · ${formatBytes(progress.bytesPerSecond)}/с`
                : ""}
            </span>
            <span class="grow"></span>
            <span class="progress-label">{progressLabel(progress)}</span>
          </div>
        </div>
      {/if}
    </div>
    <div class="actions row">
      <Button variant="ghost" onclick={() => api.openInBrowser(update.releaseUrl)}>Что нового</Button>
      <Button variant="primary" loading={updates.downloading} disabled={updates.openedInstaller != null} onclick={() => updates.install()}>
        {#if updates.downloading}
          Скачиваем…
        {:else if updates.openedInstaller}
          Установщик открыт
        {:else if !update.asset}
          Открыть релиз
        {:else}
          Скачать и установить
        {/if}
      </Button>
      <Button variant="ghost" disabled={updates.downloading} onclick={() => updates.dismiss()}>Позже</Button>
    </div>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 9px 20px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .headline {
    font-size: 12.5px;
    font-weight: 600;
  }
  .status {
    font-size: 10.5px;
    color: var(--ink-2);
    white-space: pre-wrap;
  }
  .status.error {
    color: var(--danger);
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin-top: 5px;
    font-size: 9.5px;
    color: var(--ink-2);
    word-break: break-all;
  }
  .progress {
    margin-top: 7px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .progress-row {
    font-size: 9.5px;
  }
  .progress-label {
    font-weight: 650;
  }
  .actions {
    flex: none;
    gap: 6px;
  }
</style>
