<script lang="ts">
  import { untrack } from "svelte";
  import { DATE_PRESET_LABEL } from "../format";
  import type { DashboardDateFilter, DashboardDatePreset } from "../types";

  let { value, onchange }: { value: DashboardDateFilter; onchange: (filter: DashboardDateFilter) => void } = $props();

  const presets: DashboardDatePreset[] = ["Live", "Today", "Yesterday", "Custom"];
  const initial = untrack(() => value);
  let from = $state(initial.fromDate);
  let to = $state(initial.toDate);
  let custom = $state(initial.preset === "Custom");

  function select(preset: DashboardDatePreset) {
    custom = preset === "Custom";
    if (!custom) onchange({ preset, fromDate: "", toDate: "" });
    else if (from && to) apply();
  }

  function apply() {
    if (from && to) onchange({ preset: "Custom", fromDate: from, toDate: to });
  }
</script>

<div class="picker">
  {#each presets as preset (preset)}
    <button type="button" class="option" class:active={custom ? preset === "Custom" : value.preset === preset} onclick={() => select(preset)}>
      {DATE_PRESET_LABEL[preset]}
    </button>
  {/each}
  {#if custom}
    <input type="date" bind:value={from} onchange={apply} aria-label="С" />
    <span class="faint">—</span>
    <input type="date" bind:value={to} onchange={apply} aria-label="По" />
  {/if}
</div>

<style>
  .picker {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .option {
    height: 26px;
    padding: 0 9px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--ink-2);
    font-size: var(--text-md);
  }
  .option:hover {
    color: var(--ink);
  }
  .option.active {
    color: var(--ink);
    background: var(--surface-3);
    font-weight: 500;
  }
  input {
    height: 26px;
    margin-left: 6px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: var(--text-md);
    font-family: var(--font-mono);
  }
  .faint {
    margin-left: 6px;
  }
</style>
