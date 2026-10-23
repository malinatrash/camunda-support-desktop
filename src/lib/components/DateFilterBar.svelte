<script lang="ts">
  import { untrack } from "svelte";
  import Button from "./Button.svelte";
  import SegmentedTabs from "./SegmentedTabs.svelte";
  import TextField from "./TextField.svelte";
  import { expand } from "../motion";
  import { DATE_PRESET_LABEL } from "../format";
  import type { DashboardDateFilter, DashboardDatePreset } from "../types";

  let { applied, onapply, label = "Статистика", liveLabel, hint }: {
    applied: DashboardDateFilter;
    onapply: (filter: DashboardDateFilter) => void;
    label?: string;
    liveLabel?: string;
    hint?: string;
  } = $props();

  let preset = $state<DashboardDatePreset>(untrack(() => applied.preset));
  let fromDate = $state(untrack(() => applied.fromDate));
  let toDate = $state(untrack(() => applied.toDate));

  const presets = $derived(
    (["Live", "Today", "Yesterday", "Custom"] as DashboardDatePreset[]).map((id) => ({
      id,
      label: id === "Live" && liveLabel ? liveLabel : DATE_PRESET_LABEL[id],
    })),
  );

  function selectPreset(next: DashboardDatePreset) {
    preset = next;
    if (next !== "Custom") onapply({ preset: next, fromDate: "", toDate: "" });
  }

  function applyCustom() {
    onapply({ preset: "Custom", fromDate, toDate });
  }
</script>

<div class="bar card">
  <div class="row main">
    <span class="muted label">{label}:</span>
    <SegmentedTabs items={presets} selected={preset} onselect={selectPreset} size="sm" tone="primary" />
    <span class="grow"></span>
    {#if hint}<span class="faint hint">{hint}</span>{/if}
  </div>
  {#if preset === "Custom"}
    <div class="row custom" transition:expand={{ duration: 220 }}>
      <TextField size="sm" label="С" placeholder="ГГГГ-ММ-ДД" bind:value={fromDate} onenter={applyCustom} mono class="date" />
      <TextField size="sm" label="По" placeholder="ГГГГ-ММ-ДД" bind:value={toDate} onenter={applyCustom} mono class="date" />
      <Button variant="primary" size="sm" onclick={applyCustom}>Применить</Button>
    </div>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 9px 14px;
  }
  .main {
    gap: 10px;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
  }
  .hint {
    font-size: 10px;
  }
  .custom {
    gap: 8px;
    align-items: flex-end;
  }
  .custom :global(.date) {
    width: 150px;
  }
</style>
