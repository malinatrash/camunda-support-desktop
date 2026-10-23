<script lang="ts">
  import Button from "./Button.svelte";
  import Modal from "./Modal.svelte";
  import TextField from "./TextField.svelte";
  import Spinner from "./Spinner.svelte";
  import { app } from "../state/app.svelte";
  import { api, errorMessage } from "../api";
  import { deriveCockpitUrl, isValidConnection, validateConnectionDraft } from "../connection";
  import { ENVIRONMENTS, ENVIRONMENT_LABEL, environmentTone } from "../format";
  import { expand } from "../motion";
  import type { ConnectionDraft, Environment } from "../types";

  let draft = $state<ConnectionDraft>({ name: "", restUrl: "", cockpitUrl: "", environment: "Test" });
  let submitted = $state(false);
  let checking = $state(false);
  let pingError = $state<string | null>(null);
  let cockpitOverridden = $state(false);

  const validation = $derived(validateConnectionDraft(draft));

  function reset() {
    draft = { name: "", restUrl: "", cockpitUrl: "", environment: "Test" };
    submitted = false;
    checking = false;
    pingError = null;
    cockpitOverridden = false;
  }

  $effect(() => {
    if (app.isConnectionDialogOpen) reset();
  });

  function onRestUrlInput(value: string) {
    if (!cockpitOverridden) draft.cockpitUrl = deriveCockpitUrl(value);
    pingError = null;
  }

  function selectEnvironment(environment: Environment) {
    draft.environment = environment;
  }

  async function submit() {
    submitted = true;
    pingError = null;
    if (!isValidConnection(validation)) return;
    checking = true;
    try {
      const result = await api.testConnection(draft.restUrl);
      await app.addConnection({ ...draft }, result.engineVersion);
    } catch (error) {
      pingError = errorMessage(error);
    } finally {
      checking = false;
    }
  }

  function close() {
    if (!checking) app.closeConnectionDialog();
  }
</script>

<Modal
  open={app.isConnectionDialogOpen}
  title="Новое подключение"
  subtitle="Настройки подключения хранятся только на этом компьютере."
  width="600px"
  onclose={close}
  closable={!checking}
>
  <form class="form" onsubmit={(event) => { event.preventDefault(); void submit(); }}>
    <TextField
      label="Название подключения"
      placeholder="B2C Продакшен"
      bind:value={draft.name}
      error={submitted ? validation.nameError : null}
      autofocus
    />
    <TextField
      label="Адрес Camunda REST"
      placeholder="https://camunda.example.kz/engine-rest"
      bind:value={draft.restUrl}
      oninput={onRestUrlInput}
      error={submitted ? validation.restUrlError : null}
      mono
    />
    <TextField
      label="Адрес Cockpit — создан автоматически"
      placeholder="https://camunda.example.kz/camunda/app/cockpit"
      bind:value={draft.cockpitUrl}
      oninput={() => (cockpitOverridden = true)}
      error={submitted ? validation.cockpitUrlError : null}
      mono
    />
    <div class="row hint-row">
      <span class="faint grow">Создаётся из домена REST. Изменяйте только при нестандартном адресе Cockpit.</span>
      {#if cockpitOverridden}
        <Button size="sm" onclick={() => { cockpitOverridden = false; draft.cockpitUrl = deriveCockpitUrl(draft.restUrl); }}>
          Вернуть авто
        </Button>
      {/if}
    </div>

    <div class="col env-block">
      <span class="eyebrow">Окружение</span>
      <div class="env-options">
        {#each ENVIRONMENTS as environment (environment)}
          <button
            type="button"
            class="env-option tone-{environmentTone(environment)}"
            class:selected={draft.environment === environment}
            onclick={() => selectEnvironment(environment)}
          >
            {ENVIRONMENT_LABEL[environment].short}
          </button>
        {/each}
      </div>
    </div>

    {#if checking}
      <div class="row" transition:expand>
        <Spinner size={16} label="Проверяем /engine-rest/version…" />
      </div>
    {/if}
    {#if pingError}
      <div class="message message-danger selectable" transition:expand>{pingError}</div>
    {/if}
  </form>

  {#snippet footer()}
    <Button variant="ghost" disabled={checking} onclick={close}>Отмена</Button>
    <Button variant="primary" loading={checking} onclick={submit}>{checking ? "Проверяем…" : "Проверить и добавить"}</Button>
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .hint-row {
    margin-top: -6px;
    font-size: 10.5px;
  }
  .env-block {
    gap: 8px;
    margin-top: 4px;
  }
  .env-options {
    display: flex;
    gap: 6px;
  }
  .env-option {
    padding: 7px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--ink-2);
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.05em;
    transition:
      background-color var(--dur-fast) var(--ease-out),
      border-color var(--dur-fast) var(--ease-out),
      color var(--dur-fast),
      transform var(--dur-fast) var(--ease-spring);
  }
  .env-option:hover {
    border-color: var(--border-strong);
  }
  .env-option.tone-danger.selected { color: var(--danger); background: var(--danger-soft); border-color: var(--danger); }
  .env-option.tone-warning.selected { color: var(--warning); background: var(--warning-soft); border-color: var(--warning); }
  .env-option.tone-development.selected { color: var(--info); background: var(--info-soft); border-color: var(--info); }
  .env-option.tone-neutral.selected { color: var(--neutral); background: var(--neutral-soft); border-color: var(--neutral); }
</style>
