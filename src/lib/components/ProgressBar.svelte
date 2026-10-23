<script lang="ts">
  let { fraction, tone = "primary" }: { fraction: number | null; tone?: "primary" | "danger" } = $props();
</script>

<div class="track" class:indeterminate={fraction == null}>
  <div class="fill tone-{tone}" style:width={fraction == null ? "40%" : `${Math.round(fraction * 1000) / 10}%`}></div>
</div>

<style>
  .track {
    position: relative;
    height: 6px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 999px;
    transition: width var(--dur-base) var(--ease-out);
  }
  .tone-primary {
    background: linear-gradient(90deg, var(--accent), var(--accent-strong));
  }
  .tone-danger {
    background: var(--danger);
  }
  .indeterminate .fill {
    position: absolute;
    animation: slide 1.2s var(--ease-in-out) infinite;
  }
  @keyframes slide {
    0% { left: -40%; }
    100% { left: 100%; }
  }
</style>
