<script lang="ts">
  let { size = 20, label }: { size?: number; label?: string } = $props();
</script>

<span class="spinner-wrap">
  <span class="ring" style:width="{size}px" style:height="{size}px" aria-hidden="true"></span>
  {#if label}<span class="label muted">{label}</span>{/if}
</span>

<style>
  .spinner-wrap {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .ring {
    display: inline-block;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, var(--accent) 25%, transparent);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  .label {
    font-size: 12px;
  }
</style>
