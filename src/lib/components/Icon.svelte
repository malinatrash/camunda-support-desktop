<script lang="ts">
  const PATHS: Record<string, string> = {
    monitor: "M3 5h18v11H3z M8 21h8 M12 16v5",
    search: "M11 4a7 7 0 1 1 0 14a7 7 0 0 1 0-14z M20 20l-3.5-3.5",
    alert: "M12 3l10 18H2L12 3z M12 10v4 M12 17.5v.5",
    plug: "M4 7h16v10H4z M8 3v4 M16 3v4 M9 21h6",
    settings: "M12 8.5a3.5 3.5 0 1 1 0 7a3.5 3.5 0 0 1 0-7z M4.5 12h-2 M21.5 12h-2 M12 4.5v-2 M12 21.5v-2 M6.7 6.7L5.3 5.3 M18.7 18.7l-1.4-1.4 M6.7 17.3l-1.4 1.4 M18.7 5.3l-1.4 1.4",
    sun: "M12 8a4 4 0 1 1 0 8a4 4 0 0 1 0-8z M12 2v2 M12 20v2 M2 12h2 M20 12h2 M4.9 4.9l1.4 1.4 M17.7 17.7l1.4 1.4 M4.9 19.1l1.4-1.4 M17.7 6.3l1.4-1.4",
    moon: "M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z",
    x: "M6 6l12 12 M18 6L6 18",
    chevron: "M6 9l6 6 6-6",
    chevronRight: "M9 6l6 6-6 6",
    copy: "M9 9h11v11H9z M5 15H4V4h11v1",
    external: "M14 4h6v6 M20 4l-9 9 M19 14v6H4V5h6",
    refresh: "M20 12a8 8 0 1 1-2.3-5.7 M20 4v4h-4",
    play: "M7 5l12 7-12 7z",
    pause: "M7 5h4v14H7z M13 5h4v14h-4z",
    zap: "M13 2L4 14h7l-1 8 9-12h-7z",
    clock: "M12 3a9 9 0 1 1 0 18a9 9 0 0 1 0-18z M12 8v5l3 2",
    check: "M5 12l5 5L20 7",
    star: "M12 3l2.7 5.8 6.3.7-4.7 4.3 1.3 6.2L12 17l-5.6 3 1.3-6.2L3 9.5l6.3-.7z",
    link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1 M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    command: "M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z",
    arrowLeft: "M19 12H5 M11 18l-6-6 6-6",
    list: "M4 6h16 M4 12h16 M4 18h16",
    grid: "M4 4h7v7H4z M13 4h7v7h-7z M4 13h7v7H4z M13 13h7v7h-7z",
    layers: "M12 3l9 5-9 5-9-5z M3 13l9 5 9-5",
    file: "M6 3h8l4 4v14H6z M14 3v4h4",
    plus: "M12 5v14 M5 12h14",
    filter: "M3 5h18l-7 8v6l-4 2v-8z",
    history: "M4 12a8 8 0 1 0 2.3-5.7 M4 4v4h4 M12 8v4l3 2",
    inbox: "M3 13l3-8h12l3 8v6H3z M3 13h5l2 3h4l2-3h5",
    bolt: "M13 2L4 14h7l-1 8 9-12h-7z",
    eye: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7-10-7-10-7z M12 9a3 3 0 1 1 0 6a3 3 0 0 1 0-6z",
    pin: "M9 4h6l-1 6 3 3v2H7v-2l3-3z M12 15v6",
    target: "M12 3a9 9 0 1 1 0 18a9 9 0 0 1 0-18z M12 8a4 4 0 1 1 0 8a4 4 0 0 1 0-8z M12 11v2",
  };

  let { name, size = 16, stroke = 1.75, class: className = "" }: { name: string; size?: number; stroke?: number; class?: string } = $props();
</script>

<svg
  class="icon {className}"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name] ?? ""} />
</svg>

<style>
  .icon {
    flex: none;
    display: inline-block;
    vertical-align: middle;
  }
</style>
