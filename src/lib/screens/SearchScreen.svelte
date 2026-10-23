<script lang="ts">
  import Button from "../components/Button.svelte";
  import Combobox, { type ComboboxItem } from "../components/Combobox.svelte";
  import EmptyPanel from "../components/EmptyPanel.svelte";
  import SectionTitle from "../components/SectionTitle.svelte";
  import SegmentedTabs from "../components/SegmentedTabs.svelte";
  import Spinner from "../components/Spinner.svelte";
  import StateBadge from "../components/StateBadge.svelte";
  import TextField from "../components/TextField.svelte";
  import VirtualList from "../components/VirtualList.svelte";
  import Icon from "../components/Icon.svelte";
  import { app } from "../state/app.svelte";
  import { palette } from "../state/palette.svelte";
  import { toasts } from "../state/toasts.svelte";
  import { api, copyText, errorMessage } from "../api";
  import { VALUE_TYPE_LABEL } from "../format";
  import { expand, rise } from "../motion";
  import type { ProcessDefinitionSummary, ProcessInstanceSummary, ProcessVariableCatalog, ProcessVariableDescriptor, VariableValueType } from "../types";

  const connection = $derived(app.selectedConnection);

  let mode = $state<"key" | "variables">("key");
  let keyQuery = $state("");
  let keyResults = $state<ProcessInstanceSummary[] | null>(null);
  let keySearching = $state(false);
  let keyError = $state<string | null>(null);

  async function searchByKey() {
    if (!connection || !keyQuery.trim()) return;
    keySearching = true;
    keyError = null;
    try {
      keyResults = await api.searchInstancesByBusinessKey(connection, keyQuery);
    } catch (failure) {
      keyError = errorMessage(failure);
    } finally {
      keySearching = false;
    }
  }

  $effect(() => {
    palette.refreshSerial;
    if (mode === "key" && keyResults) void searchByKey();
  });

  let definitions = $state<ProcessDefinitionSummary[]>([]);
  let definitionsLoading = $state(false);
  let definitionsError = $state<string | null>(null);
  let selectedDefinition = $state<ProcessDefinitionSummary | null>(null);

  let catalog = $state<ProcessVariableCatalog | null>(null);
  let variablesLoading = $state(false);
  let variablesError = $state<string | null>(null);
  let selectedVariable = $state<ProcessVariableDescriptor | null>(null);
  let refreshToken = $state(0);

  let variableValue = $state("");
  let valueType = $state<VariableValueType>("Auto");
  let results = $state<ProcessInstanceSummary[] | null>(null);
  let searching = $state(false);
  let error = $state<string | null>(null);

  const valueTypes: VariableValueType[] = ["Auto", "String", "Number", "Boolean"];

  const definitionItems = $derived<ComboboxItem[]>(
    definitions.map((definition) => ({
      id: definition.id,
      title: definition.name?.trim() ? definition.name : definition.key,
      subtitle: `${definition.key} · v${definition.version}`,
    })),
  );
  const variableItems = $derived<ComboboxItem[]>(
    (catalog?.variables ?? []).map((variable) => ({
      id: variable.name,
      title: variable.name,
      subtitle: `${variable.types.join(", ")} · встречается ${variable.occurrences} раз`,
      mono: true,
    })),
  );

  $effect(() => {
    const current = connection;
    definitions = [];
    selectedDefinition = null;
    catalog = null;
    selectedVariable = null;
    results = null;
    definitionsError = null;
    if (!current) return;
    let cancelled = false;
    definitionsLoading = true;
    api
      .loadSearchProcessDefinitions(current)
      .then((loaded) => {
        if (!cancelled) definitions = loaded;
      })
      .catch((failure) => {
        if (!cancelled) definitionsError = errorMessage(failure);
      })
      .finally(() => {
        if (!cancelled) definitionsLoading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const current = connection;
    const definition = selectedDefinition;
    refreshToken;
    catalog = null;
    selectedVariable = null;
    variablesError = null;
    results = null;
    if (!current || !definition) return;
    let cancelled = false;
    variablesLoading = true;
    api
      .loadProcessVariableCatalog(current, definition.key)
      .then((loaded) => {
        if (!cancelled) catalog = loaded;
      })
      .catch((failure) => {
        if (!cancelled) variablesError = errorMessage(failure);
      })
      .finally(() => {
        if (!cancelled) variablesLoading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  async function refreshVariables() {
    if (!connection || !selectedDefinition) return;
    await api.invalidateProcessVariableCatalog(connection, selectedDefinition.key);
    refreshToken += 1;
  }

  async function search() {
    if (!connection) return;
    if (!selectedDefinition) {
      error = "Сначала выберите процесс.";
      return;
    }
    if (!selectedVariable) {
      error = "Выберите переменную из списка процесса.";
      return;
    }
    if (!variableValue.trim()) {
      error = "Укажите точное значение переменной.";
      return;
    }
    searching = true;
    error = null;
    try {
      results = await api.searchProcessInstances(connection, selectedDefinition.key, selectedVariable.name, variableValue, valueType);
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      searching = false;
    }
  }

  async function copyBusinessKey(businessKey: string) {
    if (await copyText(businessKey)) toasts.success(`Бизнес-ключ ${businessKey} скопирован`);
    else toasts.error("Не удалось скопировать бизнес-ключ");
  }

  const stepText = $derived.by(() => {
    if (!selectedDefinition) return "1. Выберите процесс";
    if (variablesLoading) return "2. Читаем переменные активных заявок…";
    if (catalog) return `2. Найдено ключей: ${catalog.variables.length} в ${catalog.inspectedInstanceCount} активных заявках · кэш между сессиями до 24 ч.`;
    return "2. Выберите переменную";
  });
</script>

<div class="page">
  <SectionTitle title="Поиск заявок" description="По бизнес-ключу или ID среди запущенных и завершённых заявок, либо по точному значению переменной.">
    <SegmentedTabs items={[{ id: "key", label: "Бизнес-ключ / ID" }, { id: "variables", label: "По переменным" }]} selected={mode} onselect={(id) => (mode = id)} size="sm" tone="primary" />
  </SectionTitle>

  {#if !connection}
    <EmptyPanel title="Требуется подключение" description="Выберите или добавьте подключение Camunda перед поиском заявок." actionLabel="Добавить подключение" onaction={() => app.openConnectionDialog()} />
  {:else if mode === "key"}
    <div class="card key-form" in:rise>
      <div class="row">
        <TextField class="grow" size="md" placeholder="Бизнес-ключ целиком или его часть, либо ID заявки" bind:value={keyQuery} onenter={searchByKey} mono autofocus />
        <Button variant="primary" loading={keySearching} onclick={searchByKey}>{keySearching ? "Ищем…" : "Найти"}</Button>
      </div>
      <span class="muted small">Ищет по подстроке в runtime и History API, до 50 результатов в каждом. Та же строка доступна из любого экрана: <span class="kbd">⌘</span> <span class="kbd">K</span></span>
    </div>
    {#if keyError}<div class="message message-danger selectable" transition:expand>{keyError}</div>{/if}
    {#if keyResults}
      {#if keyResults.length === 0}
        <EmptyPanel title="Ничего не найдено" description="Проверьте ключ: поиск чувствителен к регистру так же, как Camunda." compact />
      {:else}
        <div class="card table" in:rise={{ delay: 40 }}>
          <div class="table-head">
            <span class="cell" style:flex="0.8">Состояние</span>
            <span class="cell" style:flex="1.4">Бизнес-ключ</span>
            <span class="cell" style:flex="1.3">Процесс</span>
            <span class="cell" style:flex="1.5">ID экземпляра</span>
            <span class="cell" style:flex="1.1">Старт</span>
            <span class="cell" style:flex="0.5"></span>
          </div>
          <div class="table-body">
            <VirtualList items={keyResults} itemHeight={48} key={(instance) => instance.id}>
              {#snippet row(instance)}
                <div class="table-row clickable instance-row" role="button" tabindex="0" onclick={() => app.openProcessInstance(instance.id)} onkeydown={(event) => event.key === "Enter" && app.openProcessInstance(instance.id)}>
                  <span class="cell" style:flex="0.8"><StateBadge suspended={instance.suspended} ended={instance.ended} labels="instance" /></span>
                  <span class="cell bold truncate selectable" style:flex="1.4">{instance.businessKey ?? "—"}</span>
                  <span class="cell col" style:flex="1.3"><span class="truncate">{instance.definitionName ?? instance.definitionKey}</span><span class="faint tiny mono truncate">{instance.definitionId}</span></span>
                  <span class="cell mono muted small truncate selectable" style:flex="1.5">{instance.id}</span>
                  <span class="cell muted small truncate" style:flex="1.1">{instance.startTime?.slice(0, 19).replace("T", " ") ?? "—"}</span>
                  <span class="cell" style:flex="0.5"><Icon name="chevronRight" size={14} /></span>
                </div>
              {/snippet}
            </VirtualList>
          </div>
        </div>
      {/if}
    {:else}
      <EmptyPanel title="Введите бизнес-ключ" description="Например, номер заявки из обращения клиента. Завершённые заявки тоже найдутся — через History API." />
    {/if}
  {:else}
    <div class="card form" in:rise>
      <div class="row fields">
        <div class="grow picker">
          <Combobox
            label="Процесс"
            items={definitionItems}
            selectedId={selectedDefinition?.id ?? null}
            onselect={(item) => { selectedDefinition = definitions.find((d) => d.id === item.id) ?? null; error = null; }}
            placeholder="Выберите процесс"
            filterPlaceholder="Фильтр по названию или ключу"
            loading={definitionsLoading}
            loadingText="Загружаем процессы…"
            errorText={definitionsError}
          />
        </div>
        <div class="grow picker">
          <Combobox
            label="Переменная"
            items={variableItems}
            selectedId={selectedVariable?.name ?? null}
            onselect={(item) => { selectedVariable = catalog?.variables.find((v) => v.name === item.id) ?? null; error = null; }}
            placeholder={!selectedDefinition ? "Сначала выберите процесс" : catalog && catalog.variables.length === 0 ? "У активных заявок нет переменных" : "Выберите ключ из списка"}
            filterPlaceholder="Точный фильтр по названию ключа"
            disabled={!selectedDefinition}
            loading={variablesLoading}
            loadingText="Читаем ключи…"
            errorText={variablesError}
          />
        </div>
        <TextField class="value" label="Точное значение" placeholder="ID заявки или другое значение" bind:value={variableValue} oninput={() => (error = null)} onenter={search} mono />
        <div class="col type">
          <span class="eyebrow">Тип</span>
          <div class="types">
            {#each valueTypes as type (type)}
              <button type="button" class="type-option" class:active={valueType === type} onclick={() => (valueType = type)}>{VALUE_TYPE_LABEL[type]}</button>
            {/each}
          </div>
        </div>
        <Button variant="primary" loading={searching} onclick={search}>{searching ? "Ищем…" : "Найти"}</Button>
      </div>
      <div class="row status">
        <span class="muted small">{stepText}</span>
        {#if catalog?.instancesTruncated}<span class="warning small">· список построен по первым 100 активным заявкам</span>{/if}
        <button type="button" class="link small" disabled={!selectedDefinition || variablesLoading} onclick={refreshVariables}>Обновить ключи</button>
        <span class="grow"></span>
        <span class="mono faint tiny">{connection.name} · POST /process-instance · точное совпадение</span>
      </div>
    </div>

    {#if error}
      <div class="message message-danger selectable" transition:expand>{error}</div>
    {/if}

    {#if searching}
      <div transition:expand><Spinner size={16} label="Ищем заявки выбранного процесса…" /></div>
    {/if}

    {#if results}
      <div class="col" in:rise>
        <span class="results-title">Найдено активных заявок: {results.length}</span>
        <span class="muted small">{selectedDefinition?.name?.trim() || selectedDefinition?.key || "Процесс"} · {selectedVariable?.name} = {variableValue} · тип: {VALUE_TYPE_LABEL[valueType]}</span>
      </div>
      {#if results.length === 0}
        <EmptyPanel title="Заявки не найдены" description="Для выбранного процесса нет активных заявок с таким точным значением." compact />
      {:else}
        <div class="card table" in:rise={{ delay: 40 }}>
          <div class="table-head">
            <span class="cell" style:flex="0.75">Состояние</span>
            <span class="cell" style:flex="1.4">Бизнес-ключ · клик = копия</span>
            <span class="cell" style:flex="1.35">Процесс</span>
            <span class="cell" style:flex="1.5">ID экземпляра</span>
            <span class="cell" style:flex="0.8">Тенант</span>
            <span class="cell" style:flex="0.65">Действие</span>
          </div>
          <div class="table-body">
            <VirtualList items={results} itemHeight={48} key={(instance) => instance.id}>
              {#snippet row(instance)}
                <div class="table-row clickable instance-row" role="button" tabindex="0" onclick={() => app.openProcessInstance(instance.id)} onkeydown={(event) => event.key === "Enter" && app.openProcessInstance(instance.id)}>
                  <span class="cell" style:flex="0.75"><StateBadge suspended={instance.suspended} ended={instance.ended} /></span>
                  <span class="cell" style:flex="1.4">
                    {#if instance.businessKey}
                      <button type="button" class="link truncate" onclick={(event) => { event.stopPropagation(); copyBusinessKey(instance.businessKey!); }}>{instance.businessKey}</button>
                    {:else}
                      <span class="muted">—</span>
                    {/if}
                  </span>
                  <span class="cell col" style:flex="1.35">
                    <span class="bold truncate">{instance.definitionKey}</span>
                    <span class="faint tiny truncate">{instance.definitionId}</span>
                  </span>
                  <span class="cell mono muted small truncate" style:flex="1.5">{instance.id}</span>
                  <span class="cell muted truncate" style:flex="0.8">{instance.tenantId ?? "—"}</span>
                  <span class="cell primary bold" style:flex="0.65">Открыть →</span>
                </div>
              {/snippet}
            </VirtualList>
          </div>
        </div>
      {/if}
    {:else if !searching && !error}
      <EmptyPanel title="Начните с выбора процесса" description="После выбора приложение прочитает переменные его активных заявок и покажет конкретный список ключей." />
    {/if}
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    height: 100%;
  }
  .form {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .fields {
    gap: 10px;
    align-items: flex-end;
  }
  .picker {
    flex: 1.15;
  }
  .fields :global(.value) {
    flex: 1;
  }
  .type {
    gap: 4px;
  }
  .types {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    background: var(--surface-3);
    height: 32px;
  }
  .type-option {
    border: 0;
    background: transparent;
    border-radius: 7px;
    padding: 0 9px;
    font-size: 11px;
    font-weight: 500;
    color: var(--ink-2);
    transition: background-color var(--dur-fast), color var(--dur-fast), box-shadow var(--dur-fast);
  }
  .type-option.active {
    background: var(--surface);
    color: var(--accent-ink);
    font-weight: 650;
    box-shadow: var(--shadow-1);
  }
  .status {
    gap: 10px;
  }
  .small {
    font-size: 11px;
  }
  .tiny {
    font-size: 9.5px;
  }
  .bold {
    font-weight: 600;
  }
  .warning {
    color: var(--warning);
  }
  .primary {
    color: var(--accent);
  }
  .results-title {
    font-size: 16px;
    font-weight: 650;
  }
  .table {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
    overflow: hidden;
  }
  .table-body {
    flex: 1;
    min-height: 0;
  }
  .key-form {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .instance-row {
    height: 48px;
  }
</style>
