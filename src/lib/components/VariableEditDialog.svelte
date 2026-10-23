<script lang="ts">
  import Button from "./Button.svelte";
  import Checkbox from "./Checkbox.svelte";
  import Modal from "./Modal.svelte";
  import TextField from "./TextField.svelte";
  import { validateVariableValue, variableEditHint } from "../format";
  import { expand } from "../motion";
  import type { ProcessVariable } from "../types";

  let { variable, production, busy, error, onsave, onclose }: {
    variable: ProcessVariable | null;
    production: boolean;
    busy: boolean;
    error: string | null;
    onsave: (value: string) => void;
    onclose: () => void;
  } = $props();

  let value = $state("");
  let confirmed = $state(false);

  $effect(() => {
    if (variable) {
      value = variable.value;
      confirmed = false;
    }
  });

  const validation = $derived(variable ? validateVariableValue(variable, value) : null);
  const changed = $derived(variable != null && value !== variable.value);
</script>

<Modal open={variable != null} title="Изменить переменную" subtitle="Значение будет записано непосредственно в активный экземпляр процесса." width="760px" onclose={onclose} closable={!busy}>
  {#if variable}
    <div class="summary row">
      <div class="col grow">
        <span class="mono bold">{variable.name}</span>
        <span class="faint small">Имя переменной</span>
      </div>
      <div class="col type">
        <span class="mono bold">{variable.type}</span>
        <span class="faint small">Тип сохраняется</span>
      </div>
    </div>
    <TextField label="Новое значение" bind:value multiline rows={10} mono disabled={busy} error={validation} hint={variableEditHint(variable)} />
    {#if error}<div class="message message-danger selectable" transition:expand>{error}</div>{/if}
    {#if production}
      <Checkbox bind:checked={confirmed} tone="danger" disabled={busy}>Подтверждаю изменение переменной в ПРОДАКШЕНЕ</Checkbox>
    {/if}
  {/if}
  {#snippet footer()}
    <Button variant="ghost" disabled={busy} onclick={onclose}>Отмена</Button>
    <Button variant="primary" loading={busy} disabled={!changed || validation != null || (production && !confirmed)} onclick={() => onsave(value)}>
      {busy ? "Сохраняем…" : "Сохранить значение"}
    </Button>
  {/snippet}
</Modal>

<style>
  .summary {
    padding: 12px;
    border-radius: 9px;
    background: var(--accent-soft);
  }
  .type {
    width: 180px;
  }
  .bold {
    font-weight: 700;
  }
  .small {
    font-size: 10px;
  }
</style>
