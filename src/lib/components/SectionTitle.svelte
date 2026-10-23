<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, description, children }: { title: string; description: string; children?: Snippet } = $props();
</script>

<div class="section-title">
  <div class="col grow">
    <h1 class="title">{title}</h1>
    <p class="description muted">{description}</p>
  </div>
  {#if children}
    <div class="actions row">{@render children()}</div>
  {/if}
</div>

<style>
  .section-title {
    display: flex;
    align-items: flex-end;
    gap: 12px;
  }
  .title {
    font-size: 21px;
    font-weight: 680;
    letter-spacing: -0.02em;
    line-height: 1.2;
  }
  .description {
    margin-top: 3px;
    font-size: 12px;
  }
  .actions {
    flex: none;
    gap: 8px;
  }
</style>
