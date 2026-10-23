<script lang="ts">
  import Button from "./Button.svelte";
  import Checkbox from "./Checkbox.svelte";
  import Modal from "./Modal.svelte";
  import TextField from "./TextField.svelte";
  import { contains } from "../format";
  import { teleportTargets, type ActiveActivityInstance, type BpmnNode, type ProcessInstanceDetails, type TeleportRequest } from "../types";

  let { open, details, initialTargetId, production, busy, onexecute, onclose }: {
    open: boolean;
    details: ProcessInstanceDetails;
    initialTargetId: string | null;
    production: boolean;
    busy: boolean;
    onexecute: (request: TeleportRequest) => void;
    onclose: () => void;
  } = $props();

  const cancellable = $derived(details.activeActivities.filter((activity) => activity.cancellable));
  const targets = $derived(teleportTargets(details.diagram));

  let source = $state<ActiveActivityInstance | null>(null);
  let target = $state<BpmnNode | null>(null);
  let sourceQuery = $state("");
  let targetQuery = $state("");
  let annotation = $state("");
  let confirmed = $state(false);

  $effect(() => {
    if (!open) return;
    source = cancellable[0] ?? null;
    target = targets.find((node) => node.id === initialTargetId) ?? null;
    sourceQuery = "";
    targetQuery = "";
    annotation = "";
    confirmed = false;
  });

  const sources = $derived(
    cancellable.filter(
      (activity) =>
        !sourceQuery.trim() ||
        [activity.activityName, activity.activityId, activity.activityType, activity.id].some((value) => contains(value, sourceQuery)),
    ),
  );
  const filteredTargets = $derived(
    targets.filter((node) => !targetQuery.trim() || [node.name, node.id, node.type, node.topic].some((value) => contains(value, targetQuery))),
  );
  const canExecute = $derived(!busy && source != null && target != null && annotation.trim() !== "" && (!production || confirmed));
</script>

<Modal {open} title="Телепорт заявки" subtitle="Выберите точный активный экземпляр и кубик, перед которым продолжится выполнение." width="940px" height="760px" onclose={onclose} closable={!busy}>
  <div class="pickers">
    <div class="picker card">
      <span class="eyebrow primary">1 · Откуда</span>
      <span class="picker-title">Активный элемент</span>
      <TextField size="sm" placeholder="Поиск по названию или ID" bind:value={sourceQuery} type="search" />
      <div class="list scroll">
        {#each sources as activity (activity.id)}
          <button type="button" class="item" class:selected={source?.id === activity.id} onclick={() => (source = activity)}>
            <span class="radio" class:on={source?.id === activity.id}></span>
            <span class="col grow">
              <span class="item-title truncate">{activity.activityName ?? activity.activityId}</span>
              <span class="mono faint tiny clamp-2">{activity.activityType} · {activity.activityId}</span>
            </span>
          </button>
        {:else}
          <span class="muted small">Активных элементов нет</span>
        {/each}
      </div>
    </div>
    <div class="picker card">
      <span class="eyebrow primary">2 · Куда</span>
      <span class="picker-title">Целевой кубик BPMN</span>
      <TextField size="sm" placeholder="Название, ID или topic кубика" bind:value={targetQuery} type="search" />
      <span class="faint tiny">Найдено: {filteredTargets.length}</span>
      <div class="list scroll">
        {#each filteredTargets as node (node.id)}
          <button type="button" class="item" class:selected={target?.id === node.id} onclick={() => (target = node)}>
            <span class="radio" class:on={target?.id === node.id}></span>
            <span class="col grow">
              <span class="item-title truncate">{node.name ?? node.id}</span>
              <span class="mono faint tiny clamp-2">{[node.type, node.topic ? `topic: ${node.topic}` : null, node.id].filter(Boolean).join(" · ")}</span>
            </span>
          </button>
        {/each}
      </div>
    </div>
  </div>
  <TextField label="Причина или номер обращения" bind:value={annotation} disabled={busy} />
  <div class="route row">
    <span class="route-text">{source?.activityName ?? source?.activityId ?? "Источник не выбран"}&nbsp; → &nbsp;{target?.name ?? target?.id ?? "Цель не выбрана"}</span>
    <span class="grow"></span>
    <span class="mono faint tiny">cancel + startBeforeActivity</span>
  </div>
  {#if production}
    <Checkbox bind:checked={confirmed} tone="danger" disabled={busy}>Подтверждаю телепорт в ПРОДАКШЕНЕ</Checkbox>
  {/if}
  {#snippet footer()}
    <Button variant="ghost" disabled={busy} onclick={onclose}>Отмена</Button>
    <Button
      variant={production ? "danger" : "primary"}
      loading={busy}
      disabled={!canExecute}
      onclick={() =>
        source && target && onexecute({
          sourceActivityInstanceId: source.id,
          targetActivityId: target.id,
          annotation: annotation.trim(),
          skipCustomListeners: true,
          skipIoMappings: true,
        })}
    >
      {busy ? "Выполняем…" : "Выполнить телепорт"}
    </Button>
  {/snippet}
</Modal>

<style>
  .pickers {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    flex: 1;
    min-height: 0;
  }
  .picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background: var(--surface-2);
    min-height: 0;
  }
  .primary {
    color: var(--accent);
  }
  .picker-title {
    font-size: 13px;
    font-weight: 650;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-height: 120px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: var(--surface);
    text-align: left;
    transition: background-color var(--dur-fast), border-color var(--dur-fast), transform var(--dur-fast) var(--ease-spring);
  }
  .item:hover {
    border-color: var(--border-strong);
  }
  .item.selected {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  .item:active {
    transform: scale(0.99);
  }
  .radio {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border-strong);
    flex: none;
    transition: background-color var(--dur-fast), transform var(--dur-base) var(--ease-spring);
  }
  .radio.on {
    background: var(--accent);
    transform: scale(1.3);
  }
  .item-title {
    font-size: 11.5px;
  }
  .selected .item-title {
    font-weight: 650;
  }
  .small {
    font-size: 11px;
  }
  .tiny {
    font-size: 9.5px;
  }
  .route {
    padding: 10px;
    border-radius: 8px;
    background: var(--accent-soft);
  }
  .route-text {
    font-weight: 650;
    font-size: 11.5px;
  }
</style>
