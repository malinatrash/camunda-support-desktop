<script lang="ts">
  let { suspended, ended = false, labels = "definition" }: {
    suspended: boolean;
    ended?: boolean;
    labels?: "definition" | "instance";
  } = $props();

  const text = $derived(
    ended ? "ЗАВЕРШЁН" : suspended ? (labels === "instance" ? "ПРИОСТАНОВЛЕНА" : "ПРИОСТАНОВЛЕН") : labels === "instance" ? "ЗАПУЩЕНА" : "АКТИВЕН",
  );
  const tone = $derived(ended ? "neutral" : suspended ? "warning" : "healthy");
</script>

<span class="pill pill-{tone}">{text}</span>
