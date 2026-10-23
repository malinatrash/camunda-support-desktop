<script lang="ts">
  import { untrack } from "svelte";
  import { Tween } from "svelte/motion";
  import { easeOutQuint } from "../motion";

  let { value, format = (n: number) => String(Math.round(n)), duration = 520 }: {
    value: number;
    format?: (value: number) => string;
    duration?: number;
  } = $props();

  const tween = untrack(() => new Tween(value, { duration, easing: easeOutQuint }));

  $effect(() => {
    tween.target = value;
  });
</script>

<span class="mono tabular">{format(tween.current)}</span>

<style>
  .tabular {
    font-variant-numeric: tabular-nums;
  }
</style>
