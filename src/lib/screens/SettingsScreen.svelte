<script lang="ts">
  import { onMount } from "svelte";
  import Button from "../components/Button.svelte";
  import Modal from "../components/Modal.svelte";
  import ProgressBar from "../components/ProgressBar.svelte";
  import { app } from "../state/app.svelte";
  import { theme, THEME_LABEL, FONT_SIZES, DEFAULT_FONT_SIZE } from "../state/theme.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { updates } from "../state/updates.svelte";
  import { api } from "../api";
  import { environmentTone, ENVIRONMENT_LABEL, formatBytes, grouped, progressFraction, updateStageLabel } from "../format";
  import type { AppThemeMode, CamundaConnection } from "../types";

  let pendingDelete = $state<CamundaConnection | null>(null);
  let downloads = $state<number | null>(null);
  let checking = $state(false);

  const themes: AppThemeMode[] = ["Light", "Dark"];
  const update = $derived(updates.available);

  onMount(() => {
    api
      .loadReleaseDownloadStats(false)
      .then((stats) => (downloads = stats.totalDownloads))
      .catch(() => undefined);
  });

  async function check() {
    checking = true;
    await updates.check();
    checking = false;
  }

  async function confirmDelete() {
    const connection = pendingDelete;
    pendingDelete = null;
    if (!connection) return;
    await app.deleteConnection(connection.id);
    toasts.success(`Подключение ${connection.name} удалено.`);
  }
</script>

<div class="page scroll">
  <h1>Настройки</h1>

  <section>
    <div class="section-head">
      <h2>Подключения</h2>
      <Button size="sm" variant="ghost" onclick={() => app.openConnectionDialog()}>Добавить</Button>
    </div>
    {#each app.connections as connection (connection.id)}
      <div class="connection" class:selected={connection.id === app.selectedConnectionId}>
        <span class="env-dot tone-{environmentTone(connection.environment)}"></span>
        <button type="button" class="col grow connection-main" onclick={() => app.selectConnection(connection.id)}>
          <span class="truncate">{connection.name} <span class="faint">· {ENVIRONMENT_LABEL[connection.environment].label}{connection.engineVersion ? ` · Camunda ${connection.engineVersion}` : ""}</span></span>
          <span class="mono faint small truncate selectable">{connection.restUrl}</span>
        </button>
        <Button size="sm" variant="ghost" onclick={() => (pendingDelete = connection)}>Удалить</Button>
      </div>
    {:else}
      <p class="faint empty">Подключений пока нет</p>
    {/each}
  </section>

  <section>
    <div class="section-head"><h2>Вид</h2></div>
    <div class="setting">
      <span class="grow">Тема</span>
      <div class="choices">
        {#each themes as mode (mode)}
          <button type="button" class="choice" class:active={theme.mode === mode} onclick={() => theme.set(mode)}>{THEME_LABEL[mode]}</button>
        {/each}
      </div>
    </div>
    <div class="setting">
      <span class="col grow">
        <span>Размер шрифта</span>
        <span class="faint small">По умолчанию {DEFAULT_FONT_SIZE}</span>
      </span>
      <div class="choices">
        {#each FONT_SIZES as size (size)}
          <button type="button" class="choice mono" class:active={theme.fontSize === size} onclick={() => theme.setFontSize(size)}>{size}</button>
        {/each}
      </div>
    </div>
  </section>

  <section>
    <div class="section-head"><h2>Обновления</h2></div>
    <div class="setting">
      <span class="col grow">
        <span>Версия {updates.info.version}</span>
        <span class="faint small">
          Сборка {updates.info.build}{downloads != null ? ` · скачиваний ${grouped(downloads)}` : ""}
        </span>
      </span>
      {#if update}
        <Button size="sm" variant="ghost" onclick={() => api.openInBrowser(update.releaseUrl)}>Что нового</Button>
        <Button size="sm" variant="primary" loading={updates.downloading} disabled={updates.openedInstaller != null} onclick={() => updates.install()}>
          {updates.openedInstaller ? "Установщик открыт" : update.asset ? `Установить ${update.version}` : `Открыть релиз ${update.version}`}
        </Button>
      {:else}
        <Button size="sm" variant="ghost" loading={checking} onclick={check}>Проверить</Button>
      {/if}
    </div>
    {#if update && updates.downloading && updates.progress}
      <div class="progress">
        <ProgressBar fraction={progressFraction(updates.progress)} />
        <span class="faint small">{updateStageLabel(updates.progress.stage)} · {formatBytes(updates.progress.downloadedBytes)} из {formatBytes(updates.progress.totalBytes)}</span>
      </div>
    {/if}
    {#if updates.openedInstaller}
      <p class="small muted">Установщик {updates.openedInstaller} открыт — завершите установку в системном окне.</p>
    {/if}
    {#if updates.error ?? updates.lastCheckError}
      <p class="small danger selectable">{updates.error ?? updates.lastCheckError}</p>
    {/if}
  </section>
</div>

<Modal open={pendingDelete != null} title="Удалить подключение?" width="440px" onclose={() => (pendingDelete = null)}>
  <p>Подключение <strong>{pendingDelete?.name}</strong> будет удалено с этого компьютера. Открытые вкладки заявок этого окружения закроются.</p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (pendingDelete = null)}>Отмена</Button>
    <Button variant="danger" onclick={confirmDelete}>Удалить</Button>
  {/snippet}
</Modal>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 20px;
    height: 100%;
    max-width: 760px;
    padding-bottom: 24px;
  }
  .page > :global(*) {
    flex-shrink: 0;
  }
  h1 {
    font-size: var(--text-xl);
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  section {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    overflow: hidden;
  }
  section > :global(:last-child) {
    border-bottom: 0;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding: 0 12px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
  }
  h2 {
    font-size: var(--text-base);
    font-weight: 600;
  }
  .connection,
  .setting {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 52px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .connection:hover {
    background: var(--row-hover);
  }
  .connection-main {
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
  }
  .connection.selected .connection-main > span:first-child {
    font-weight: 600;
  }
  .env-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
    background: var(--neutral);
  }
  .env-dot.tone-danger { background: var(--danger); }
  .env-dot.tone-warning { background: var(--warning); }
  .env-dot.tone-development { background: var(--info); }
  .choices {
    display: flex;
    gap: 2px;
  }
  .choice {
    min-width: 34px;
    height: 28px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
  }
  .choice:hover {
    color: var(--ink);
  }
  .choice:hover {
    background: var(--row-hover);
  }
  .choice.active {
    background: var(--surface-3);
    color: var(--ink);
    font-weight: 500;
  }
  .small {
    font-size: var(--text-sm);
  }
  .empty {
    padding: 16px 0;
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 0;
  }
  p {
    padding-top: 8px;
  }
  .danger {
    color: var(--danger);
  }
</style>
