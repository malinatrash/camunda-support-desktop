<script lang="ts">
  import { flip } from "svelte/animate";
  import Button from "../components/Button.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import EnvironmentBadge from "../components/EnvironmentBadge.svelte";
  import SectionTitle from "../components/SectionTitle.svelte";
  import Modal from "../components/Modal.svelte";
  import { app } from "../state/app.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { environmentTone } from "../format";
  import { flipOptions, rise, pop } from "../motion";
  import type { CamundaConnection } from "../types";

  let pendingDelete = $state<CamundaConnection | null>(null);

  async function confirmDelete() {
    const connection = pendingDelete;
    pendingDelete = null;
    if (!connection) return;
    await app.deleteConnection(connection.id);
    toasts.success(`Подключение ${connection.name} удалено.`);
  }
</script>

<div class="page">
  <SectionTitle title="Подключения" description="Окружения Camunda REST, доступные с этого компьютера.">
    <Button variant="primary" onclick={() => app.openConnectionDialog()}>Добавить подключение</Button>
  </SectionTitle>

  {#if app.connections.length === 0}
    <EmptyPanel
      title="Подключения ещё не настроены"
      description="Настройки подключений хранятся локально; данные авторизации не записываются в профиль подключения."
      actionLabel="Добавить подключение"
      onaction={() => app.openConnectionDialog()}
    />
  {:else}
    <div class="list scroll">
      {#each app.connections as connection, index (connection.id)}
        {@const selected = connection.id === app.selectedConnectionId}
        <div
          class="connection card tone-{environmentTone(connection.environment)}"
          class:selected
          animate:flip={flipOptions}
          in:rise={{ delay: index * 30 }}
          out:pop={{ duration: 140 }}
          role="button"
          tabindex="0"
          onclick={() => app.selectConnection(connection.id)}
          onkeydown={(event) => event.key === "Enter" && app.selectConnection(connection.id)}
        >
          <div class="col grow">
            <div class="row">
              <span class="name">{connection.name}</span>
              <EnvironmentBadge environment={connection.environment} />
              {#if connection.engineVersion}<span class="faint engine">Camunda {connection.engineVersion}</span>{/if}
            </div>
            <span class="url mono muted truncate selectable">{connection.restUrl}</span>
            <span class="url mono faint truncate selectable">{connection.cockpitUrl}</span>
          </div>
          <Button size="sm" onclick={(event) => { event.stopPropagation(); pendingDelete = connection; }}>Удалить</Button>
        </div>
      {/each}
    </div>
  {/if}
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
    gap: 14px;
    min-height: 0;
    height: 100%;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    padding-right: 4px;
  }
  .connection {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    cursor: pointer;
    transition: border-color var(--dur-fast) var(--ease-out), box-shadow var(--dur-base) var(--ease-out), transform var(--dur-base) var(--ease-out);
  }
  .connection:hover {
    box-shadow: var(--shadow-2);
  }
  .connection.selected.tone-danger { border-color: var(--danger); box-shadow: 0 0 0 1px var(--danger); }
  .connection.selected.tone-warning { border-color: var(--warning); box-shadow: 0 0 0 1px var(--warning); }
  .connection.selected.tone-development { border-color: var(--info); box-shadow: 0 0 0 1px var(--info); }
  .connection.selected.tone-neutral { border-color: var(--neutral); box-shadow: 0 0 0 1px var(--neutral); }
  .name {
    font-weight: 650;
    font-size: 13px;
  }
  .engine {
    font-size: 10px;
  }
  .url {
    font-size: 11px;
    margin-top: 2px;
  }
</style>
