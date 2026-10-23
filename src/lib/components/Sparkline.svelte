<script lang="ts">
  let { values, tone = "accent", height = 26, width = 72 }: { values: number[]; tone?: "accent" | "danger" | "healthy" | "warning"; height?: number; width?: number } = $props();

  const points = $derived.by(() => {
    if (values.length === 0) return "";
    const max = Math.max(1, ...values);
    const step = values.length > 1 ? width / (values.length - 1) : width;
    return values.map((value, index) => `${(index * step).toFixed(1)},${(height - 2 - (value / max) * (height - 4)).toFixed(1)}`).join(" ");
  });
  const area = $derived(points ? `0,${height} ${points} ${width},${height}` : "");
</script>

<svg class="spark tone-{tone}" viewBox="0 0 {width} {height}" {width} {height} aria-hidden="true">
  <polygon class="area" points={area} />
  <polyline class="line" points={points} fill="none" />
</svg>

<style>
  .spark {
    display: block;
    overflow: visible;
  }
  .line {
    stroke-width: 1.5;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .area {
    opacity: 0;
  }
  .tone-accent .line { stroke: var(--accent); }
  .tone-accent .area { fill: var(--accent); }
  .tone-danger .line { stroke: var(--danger); }
  .tone-danger .area { fill: var(--danger); }
  .tone-healthy .line { stroke: var(--healthy); }
  .tone-healthy .area { fill: var(--healthy); }
  .tone-warning .line { stroke: var(--warning); }
  .tone-warning .area { fill: var(--warning); }
</style>
