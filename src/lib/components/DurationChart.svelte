<script lang="ts">
  import { formatDuration } from "../format";
  import type { DashboardDurationBucket } from "../types";

  let { buckets, average, median, p95 }: {
    buckets: DashboardDurationBucket[];
    average: number | null;
    median: number | null;
    p95: number | null;
  } = $props();

  const total = $derived(buckets.reduce((sum, bucket) => sum + bucket.count, 0));
  const maximum = $derived(Math.max(1, ...buckets.map((bucket) => bucket.count)));
</script>

<div class="card chart">
  <div class="row head">
    <div class="col grow">
      <span class="title">Распределение времени прохождения</span>
      <span class="faint small">Помогает увидеть длинный хвост и заявки, выходящие за ожидаемое время</span>
    </div>
    <div class="metric"><span class="eyebrow">Среднее</span><span class="metric-value mono primary">{formatDuration(average)}</span></div>
    <div class="metric"><span class="eyebrow">Медиана</span><span class="metric-value mono healthy">{formatDuration(median)}</span></div>
    <div class="metric"><span class="eyebrow">P95</span><span class="metric-value mono warning">{formatDuration(p95)}</span></div>
  </div>
  <div class="divider"></div>
  {#if total === 0}
    <div class="empty muted">Нет завершённых заявок с данными о длительности</div>
  {:else}
    <div class="bars">
      {#each buckets as bucket, index (bucket.label)}
        <div class="bucket">
          <span class="count mono" class:warning={index >= buckets.length - 2 && bucket.count > 0}>{bucket.count}</span>
          <div class="track">
            <span
              class="bar"
              class:danger={index === buckets.length - 1}
              class:warn={index === buckets.length - 2}
              style:height="{(bucket.count / maximum) * 100}%"
            ></span>
          </div>
          <span class="label faint">{bucket.label}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .chart {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 11px 14px;
    height: 190px;
  }
  .head {
    gap: 18px;
  }
  .title {
    font-weight: 650;
    font-size: 13px;
  }
  .small {
    font-size: 9.5px;
  }
  .metric {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }
  .metric-value {
    font-size: 12px;
    font-weight: 700;
  }
  .primary { color: var(--accent); }
  .healthy { color: var(--healthy); }
  .warning { color: var(--warning); }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    font-size: 11.5px;
  }
  .bars {
    flex: 1;
    display: flex;
    gap: 10px;
    min-height: 0;
  }
  .bucket {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    min-height: 0;
  }
  .count {
    font-size: 11px;
    font-weight: 700;
  }
  .track {
    flex: 1;
    width: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    min-height: 0;
  }
  .bar {
    width: 30px;
    border-radius: 5px 5px 0 0;
    background: color-mix(in srgb, var(--accent) 82%, transparent);
    transition: height var(--dur-slow) var(--ease-out);
  }
  .bar.warn { background: var(--warning); }
  .bar.danger { background: var(--danger); }
  .label {
    font-size: 9.5px;
    white-space: nowrap;
  }
</style>
