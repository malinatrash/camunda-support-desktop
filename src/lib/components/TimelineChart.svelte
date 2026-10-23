<script lang="ts">
  import { Tween } from "svelte/motion";
  import { easeOutQuint } from "../motion";
  import type { DashboardTimelinePoint } from "../types";

  let { points, title, expanded, ontoggle }: {
    points: DashboardTimelinePoint[];
    title: string;
    expanded: boolean;
    ontoggle: () => void;
  } = $props();

  const maximum = $derived(Math.max(1, ...points.map((point) => Math.max(point.started, point.completed, point.incidents))));
  const incidentLine = new Tween<number[]>([], { duration: 600, easing: easeOutQuint });

  $effect(() => {
    const next = points.map((point) => point.incidents / maximum);
    incidentLine.set(next, { duration: incidentLine.current.length === next.length ? 600 : 0 });
  });

  const polyline = $derived(
    incidentLine.current
      .map((value, index) => {
        const x = ((index + 0.5) / Math.max(1, incidentLine.current.length)) * 100;
        return `${x.toFixed(2)},${(100 - value * 100).toFixed(2)}`;
      })
      .join(" "),
  );

  const labelStep = $derived(Math.max(1, Math.round(points.length / 6)));
  const labels = $derived(points.filter((_, index) => index % labelStep === 0 || index === points.length - 1).slice(0, 7));
  let hovered = $state<number | null>(null);
</script>

<div class="chart card" class:expanded>
  <div class="head row">
    <div class="col grow">
      <span class="title">{title}</span>
      <span class="faint small">Старт, полное завершение и инциденты</span>
    </div>
    <span class="legend"><i class="sw started"></i>Старт</span>
    <span class="legend"><i class="sw completed"></i>Завершено</span>
    <span class="legend"><i class="sw incidents"></i>Инциденты</span>
    <button type="button" class="link toggle" onclick={ontoggle}>{expanded ? "Свернуть ↙" : "Развернуть ↗"}</button>
  </div>
  {#if points.length === 0}
    <div class="empty muted">За период нет исторических данных</div>
  {:else}
    <div class="plot" role="img" aria-label={title}>
      <div class="grid">
        {#each [0, 1, 2, 3] as line (line)}<span class="grid-line"></span>{/each}
      </div>
      <div class="bars">
        {#each points as point, index (index)}
          <div
            class="group"
            class:hovered={hovered === index}
            onpointerenter={() => (hovered = index)}
            onpointerleave={() => (hovered = null)}
            role="presentation"
          >
            <span class="bar started" style:height="{(point.started / maximum) * 100}%"></span>
            <span class="bar completed" style:height="{(point.completed / maximum) * 100}%"></span>
            {#if hovered === index}
              <div class="tooltip card">
                <span class="tooltip-label">{point.label}</span>
                <span class="started-text">Старт: {point.started}</span>
                <span class="completed-text">Завершено: {point.completed}</span>
                <span class="incident-text">Инциденты: {point.incidents}</span>
              </div>
            {/if}
          </div>
        {/each}
      </div>
      <svg class="line" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
        <polyline points={polyline} fill="none" stroke="var(--danger)" stroke-width="1.6" vector-effect="non-scaling-stroke" stroke-linejoin="round" stroke-linecap="round" />
      </svg>
      <svg class="dots" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
        {#each incidentLine.current as value, index (index)}
          <circle cx={((index + 0.5) / Math.max(1, incidentLine.current.length)) * 100} cy={100 - value * 100} r="0.9" fill="var(--danger)" vector-effect="non-scaling-stroke" />
        {/each}
      </svg>
    </div>
    <div class="labels">
      {#each labels as label (label.label)}
        <span class="label">{label.label.replace(" ", "\n")}</span>
      {/each}
    </div>
  {/if}
</div>

<style>
  .chart {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 13px;
    min-height: 0;
  }
  .head {
    gap: 10px;
  }
  .title {
    font-weight: 650;
    font-size: 13px;
  }
  .small {
    font-size: 9.5px;
  }
  .legend {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 9px;
    color: var(--ink-2);
  }
  .sw {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    display: inline-block;
  }
  .sw.started { background: var(--accent); }
  .sw.completed { background: var(--ink-3); }
  .sw.incidents { background: var(--danger); }
  .toggle {
    font-size: 10.5px;
    margin-left: 4px;
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    font-size: 11.5px;
    min-height: 120px;
  }
  .plot {
    position: relative;
    flex: 1;
    min-height: 140px;
  }
  .grid {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    pointer-events: none;
  }
  .grid-line {
    height: 1px;
    background: color-mix(in srgb, var(--border) 80%, transparent);
  }
  .bars {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-end;
  }
  .group {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 2px;
    position: relative;
    border-radius: 4px;
    transition: background-color var(--dur-fast);
  }
  .group.hovered {
    background: color-mix(in srgb, var(--accent-soft) 50%, transparent);
  }
  .bar {
    width: 25%;
    max-width: 12px;
    border-radius: 3px 3px 0 0;
    transition: height var(--dur-slow) var(--ease-out);
    transform-origin: bottom;
  }
  .bar.started { background: color-mix(in srgb, var(--accent) 82%, transparent); }
  .bar.completed { background: var(--surface-4); }
  .line,
  .dots {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: visible;
  }
  .tooltip {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 5;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 9px;
    font-size: 10px;
    white-space: nowrap;
    box-shadow: var(--shadow-2);
    pointer-events: none;
  }
  .tooltip-label {
    font-weight: 700;
  }
  .started-text { color: var(--accent); }
  .completed-text { color: var(--ink-2); }
  .incident-text { color: var(--danger); }
  .labels {
    display: flex;
    height: 30px;
  }
  .label {
    flex: 1;
    text-align: center;
    font-size: 10px;
    color: var(--ink-2);
    white-space: pre-line;
    line-height: 1.3;
  }
  .expanded .label {
    font-size: 11px;
  }
</style>
