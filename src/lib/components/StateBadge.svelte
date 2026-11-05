<script lang="ts">
  let { suspended, ended = false, labels = "definition" }: {
    suspended: boolean;
    ended?: boolean;
    labels?: "definition" | "instance";
  } = $props();

  const text = $derived(
    labels === "instance"
      ? ended ? "завершена" : suspended ? "приостановлена" : "в работе"
      : ended ? "завершён" : suspended ? "приостановлен" : "активен",
  );
  const tone = $derived(ended ? "neutral" : suspended ? "warning" : "healthy");
</script>

<span class="pill pill-{tone}">{text}</span>
