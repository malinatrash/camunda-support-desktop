<script lang="ts">
  import Button from "./Button.svelte";
  import Checkbox from "./Checkbox.svelte";
  import Modal from "./Modal.svelte";

  let { open, description, production, busy = false, onconfirm, oncancel }: {
    open: boolean;
    description: string;
    production: boolean;
    busy?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  let confirmed = $state(false);

  $effect(() => {
    if (open) confirmed = false;
  });
</script>

<Modal {open} title="Подтверждение операции" width="480px" onclose={oncancel} closable={!busy}>
  <p class="selectable">{description}</p>
  {#if production}
    <Checkbox bind:checked={confirmed} tone="danger">Подтверждаю выполнение операции в ПРОДАКШЕНЕ</Checkbox>
  {/if}
  {#snippet footer()}
    <Button variant="ghost" disabled={busy} onclick={oncancel}>Отмена</Button>
    <Button variant={production ? "danger" : "primary"} loading={busy} disabled={production && !confirmed} onclick={onconfirm}>Выполнить</Button>
  {/snippet}
</Modal>
