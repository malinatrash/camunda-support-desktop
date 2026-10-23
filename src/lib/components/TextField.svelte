<script lang="ts">
  import { focusOnMount } from "../actions";

  interface Props {
    value?: string;
    label?: string;
    placeholder?: string;
    error?: string | null;
    hint?: string | null;
    mono?: boolean;
    disabled?: boolean;
    multiline?: boolean;
    rows?: number;
    size?: "sm" | "md";
    type?: "text" | "search";
    autofocus?: boolean;
    onenter?: () => void;
    oninput?: (value: string) => void;
    class?: string;
  }

  let {
    value = $bindable(""),
    label,
    placeholder = "",
    error = null,
    hint = null,
    mono = false,
    disabled = false,
    multiline = false,
    rows = 6,
    size = "md",
    type = "text",
    autofocus = false,
    onenter,
    oninput,
    class: className = "",
  }: Props = $props();

  function handleKey(event: KeyboardEvent) {
    if (event.key === "Enter" && !multiline && onenter) {
      event.preventDefault();
      onenter();
    }
  }

  function handleInput(event: Event) {
    value = (event.target as HTMLInputElement | HTMLTextAreaElement).value;
    oninput?.(value);
  }
</script>

<label class="field field-{size} {className}" class:has-error={error != null} class:disabled>
  {#if label}<span class="field-label">{label}</span>{/if}
  {#if multiline}
    <textarea
      class="control selectable"
      class:mono
      {rows}
      {placeholder}
      {disabled}
      {value}
      oninput={handleInput}
      use:focusOnMount={autofocus}
    ></textarea>
  {:else}
    <input
      class="control selectable"
      class:mono
      {type}
      {placeholder}
      {disabled}
      {value}
      oninput={handleInput}
      onkeydown={handleKey}
      spellcheck="false"
      autocomplete="off"
      use:focusOnMount={autofocus}
    />
  {/if}
  {#if error}
    <span class="field-help error">{error}</span>
  {:else if hint}
    <span class="field-help">{hint}</span>
  {/if}
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .field-label {
    font-size: 10.5px;
    font-weight: 600;
    color: var(--ink-2);
    letter-spacing: 0.02em;
  }
  .control {
    width: 100%;
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--ink);
    padding: 0 10px;
    height: 32px;
    outline: none;
    transition:
      border-color var(--dur-fast) var(--ease-out),
      box-shadow var(--dur-fast) var(--ease-out),
      background-color var(--dur-fast) var(--ease-out);
  }
  .field-sm .control {
    height: 28px;
    font-size: 12px;
    padding: 0 9px;
  }
  textarea.control {
    height: auto;
    padding: 8px 10px;
    resize: vertical;
    line-height: 1.5;
    font-size: 12px;
  }
  .control::placeholder {
    color: var(--ink-3);
  }
  .control:hover:not(:disabled) {
    border-color: var(--ink-3);
  }
  .control:focus {
    border-color: var(--accent);
    box-shadow: var(--focus-ring);
  }
  .has-error .control {
    border-color: var(--danger);
  }
  .has-error .control:focus {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--danger) 25%, transparent);
  }
  .disabled .control {
    opacity: 0.6;
  }
  .field-help {
    font-size: 10.5px;
    color: var(--ink-2);
    line-height: 1.4;
  }
  .field-help.error {
    color: var(--danger);
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
