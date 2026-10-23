<script module lang="ts">
  interface CameraState {
    rootId: string | null;
    viewbox: { x: number; y: number; width: number; height: number };
  }

  // Камера (зум, положение, открытый подпроцесс) запоминается по тексту схемы и переживает обновления и смену вкладок.
  const cameraByXml = new Map<string, CameraState>();
  const CAMERA_LIMIT = 24;

  function rememberCamera(xml: string, instance: any) {
    try {
      const canvas = instance.get("canvas");
      const root = canvas.getRootElement();
      cameraByXml.delete(xml);
      cameraByXml.set(xml, { rootId: root?.id ?? null, viewbox: canvas.viewbox() });
      if (cameraByXml.size > CAMERA_LIMIT) cameraByXml.delete(cameraByXml.keys().next().value!);
    } catch {
      // Схема ещё не загружена — запоминать нечего.
    }
  }

  function restoreCamera(xml: string, instance: any): boolean {
    const saved = cameraByXml.get(xml);
    if (!saved) return false;
    try {
      const canvas = instance.get("canvas");
      const elementRegistry = instance.get("elementRegistry");
      if (saved.rootId) {
        const root = elementRegistry.get(saved.rootId);
        if (root && root !== canvas.getRootElement()) canvas.setRootElement(root);
      }
      canvas.viewbox(saved.viewbox);
      return true;
    } catch {
      return false;
    }
  }
</script>

<script lang="ts">
  import NavigatedViewer from "bpmn-js/lib/NavigatedViewer";
  import "bpmn-js/dist/assets/diagram-js.css";
  import "bpmn-js/dist/assets/bpmn-js.css";
  import "bpmn-js/dist/assets/bpmn-font/css/bpmn-embedded.css";
  import { onDestroy } from "svelte";
  import { theme } from "../state/theme.svelte";
  import { fadeFast } from "../motion";


  interface Props {
    xml: string;
    activeIds: string[];
    incidentIds: string[];
    completedCounts: Record<string, number>;
    clickableIds: string[];
    selectedId: string | null;
    onActivityClick: (activityId: string) => void;
  }

  let { xml, activeIds, incidentIds, completedCounts, clickableIds, selectedId, onActivityClick }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let viewer: any = null;
  let importedXml = "";
  let ready = $state(false);
  let error = $state<string | null>(null);
  let zoomLevel = $state(1);
  let appliedMarkers: { id: string; marker: string }[] = [];
  let clickHandler: ((event: any) => void) | null = null;

  const palette = $derived(
    theme.mode === "Dark"
      ? { defaultFillColor: "#1b2129", defaultStrokeColor: "#dfe6ee", defaultLabelColor: "#dfe6ee" }
      : { defaultFillColor: "#ffffff", defaultStrokeColor: "#1f2a24", defaultLabelColor: "#1f2a24" },
  );

  let renderedTheme = "";

  function destroyViewer() {
    if (viewer) {
      if (importedXml) rememberCamera(importedXml, viewer);
      viewer.destroy();
      viewer = null;
    }
    importedXml = "";
    appliedMarkers = [];
    ready = false;
  }

  async function createViewer(host: HTMLDivElement, colors: typeof palette, source: string) {
    destroyViewer();
    error = null;
    const instance = new NavigatedViewer({ container: host, bpmnRenderer: colors });
    viewer = instance;
    try {
      await instance.importXML(source);
      if (viewer !== instance) return;
      importedXml = source;
      renderedTheme = theme.mode;
      const canvas = instance.get("canvas") as any;
      if (!restoreCamera(source, instance)) canvas.zoom("fit-viewport", "auto");
      zoomLevel = canvas.zoom();
      const eventBus = instance.get("eventBus") as any;
      clickHandler = (event: any) => {
        const id = event?.element?.id;
        if (id && clickableIds.includes(id)) onActivityClick(id);
      };
      eventBus.on("element.click", clickHandler);
      eventBus.on("canvas.viewbox.changed", () => {
        zoomLevel = canvas.zoom();
        rememberCamera(source, instance);
      });
      eventBus.on("root.set", () => rememberCamera(source, instance));
      ready = true;
      applyMarkers();
    } catch (failure) {
      if (viewer === instance) error = failure instanceof Error ? failure.message : String(failure);
    }
  }

  // Отмечает кубик и все свёрнутые подпроцессы, внутри которых он лежит.
  function markWithParents(elementRegistry: any, canvas: any, id: string, marker: string, seen: Set<string>) {
    const element = elementRegistry.get(id);
    if (!element || seen.has(`${id}:${marker}`)) return;
    seen.add(`${id}:${marker}`);
    canvas.addMarker(id, marker);
    appliedMarkers.push({ id, marker });
    const root = canvas.findRoot(element);
    const parentId = root?.businessObject?.id;
    if (root && typeof root.id === "string" && root.id.endsWith("_plane") && parentId && parentId !== id) {
      markWithParents(elementRegistry, canvas, parentId, marker, seen);
    }
  }

  function applyMarkers() {
    if (!viewer || !ready) return;
    const canvas = viewer.get("canvas");
    const elementRegistry = viewer.get("elementRegistry");
    const overlays = viewer.get("overlays");
    for (const { id, marker } of appliedMarkers) {
      if (elementRegistry.get(id)) canvas.removeMarker(id, marker);
    }
    appliedMarkers = [];
    overlays.remove({ type: "support-count" });
    const seen = new Set<string>();
    const completedByParent = new Map<string, number>();
    for (const [id, count] of Object.entries(completedCounts)) {
      if (count < 1 || !elementRegistry.get(id)) continue;
      markWithParents(elementRegistry, canvas, id, "support-completed", seen);
      const badge = document.createElement("div");
      badge.className = "support-count-badge";
      badge.textContent = `×${count}`;
      overlays.add(id, "support-count", { position: { top: -9, right: 9 }, html: badge });
      const root = canvas.findRoot(elementRegistry.get(id));
      const parentId = root?.businessObject?.id;
      if (root && root.id.endsWith("_plane") && parentId) completedByParent.set(parentId, (completedByParent.get(parentId) ?? 0) + count);
    }
    for (const [parentId, count] of completedByParent) {
      if (!elementRegistry.get(parentId) || parentId in completedCounts) continue;
      const badge = document.createElement("div");
      badge.className = "support-count-badge";
      badge.textContent = `×${count}`;
      overlays.add(parentId, "support-count", { position: { top: -9, right: 9 }, html: badge });
    }
    for (const id of activeIds) markWithParents(elementRegistry, canvas, id, "support-active", seen);
    for (const id of incidentIds) markWithParents(elementRegistry, canvas, id, "support-incident", seen);
    for (const id of clickableIds) {
      if (elementRegistry.get(id)) {
        canvas.addMarker(id, "support-clickable");
        appliedMarkers.push({ id, marker: "support-clickable" });
      }
    }
    if (selectedId && elementRegistry.get(selectedId)) {
      canvas.addMarker(selectedId, "support-selected");
      appliedMarkers.push({ id: selectedId, marker: "support-selected" });
    }
  }

  // Пересоздаём viewer только когда реально сменилась схема или тема; обновление данных заявки камеру не трогает.
  $effect(() => {
    const host = container;
    const colors = palette;
    const source = xml;
    const mode = theme.mode;
    if (!host || !source.trim()) return;
    if (viewer && importedXml === source && renderedTheme === mode) return;
    void createViewer(host, colors, source);
  });

  onDestroy(() => destroyViewer());

  $effect(() => {
    activeIds;
    incidentIds;
    completedCounts;
    clickableIds;
    selectedId;
    ready;
    applyMarkers();
  });

  function zoomBy(factor: number) {
    if (!viewer) return;
    const canvas = viewer.get("canvas");
    canvas.zoom(Math.max(0.1, Math.min(8, canvas.zoom() * factor)));
  }

  function fit() {
    viewer?.get("canvas").zoom("fit-viewport", "auto");
  }
</script>

<div class="bpmn-host">
  <div class="bpmn-canvas" bind:this={container}></div>
  {#if !ready && !error}
    <div class="overlay" transition:fadeFast>
      <span class="loading">Подготавливаем BPMN-схему…</span>
    </div>
  {/if}
  {#if error}
    <div class="overlay" transition:fadeFast>
      <span class="failure">Не удалось открыть BPMN-схему</span>
      <span class="muted selectable">{error}</span>
    </div>
  {/if}
  <div class="zoom card">
    <button type="button" onclick={() => zoomBy(1 / 1.2)} title="Уменьшить масштаб">−</button>
    <button type="button" class="fit" onclick={fit} title="Показать схему целиком">По размеру</button>
    <button type="button" onclick={() => zoomBy(1.2)} title="Увеличить масштаб">+</button>
    <span class="level mono">{Math.round(zoomLevel * 100)}%</span>
  </div>
</div>

<style>
  .bpmn-host {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 0;
    background: var(--surface-2);
    overflow: hidden;
    contain: strict;
  }
  .bpmn-canvas {
    position: absolute;
    inset: 0;
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 12px;
    background: var(--surface-2);
    pointer-events: none;
  }
  .loading {
    color: var(--ink-2);
  }
  .failure {
    color: var(--danger);
    font-weight: 600;
  }
  .zoom {
    position: absolute;
    top: 10px;
    right: 10px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    box-shadow: var(--shadow-2);
  }
  .zoom button {
    height: 28px;
    min-width: 30px;
    padding: 0 9px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ink);
    font-weight: 600;
    font-size: 13px;
    transition: background-color var(--dur-fast);
  }
  .zoom button:hover {
    background: var(--surface-3);
  }
  .zoom .fit {
    font-size: 11px;
  }
  .level {
    font-size: 10px;
    color: var(--ink-2);
    padding: 0 6px;
    min-width: 40px;
    text-align: right;
  }

  :global(.bpmn-host .djs-container) {
    --context-pad-entry-background-color: var(--surface);
    --breadcrumbs-font-family: var(--font-sans);
  }
  :global(.bpmn-host .djs-element) {
    transition: filter var(--dur-fast) var(--ease-out);
  }
  :global(.bpmn-host .djs-element.support-clickable) {
    cursor: pointer;
  }
  :global(.bpmn-host .djs-element.support-clickable:hover .djs-visual > :first-child) {
    filter: drop-shadow(0 0 4px rgba(var(--accent-rgb), 0.6));
  }
  :global(.bpmn-host .djs-element.support-completed:not(.djs-connection) .djs-visual > :first-child) {
    stroke: #8a948e !important;
    stroke-width: 2.5px !important;
    fill: #eef2ef !important;
  }
  :global([data-theme="dark"] .bpmn-host .djs-element.support-completed:not(.djs-connection) .djs-visual > :first-child) {
    fill: #181d1b !important;
  }
  :global(.bpmn-host .djs-element.support-active:not(.djs-connection) .djs-visual > :first-child) {
    stroke: #15803d !important;
    stroke-width: 3.5px !important;
    fill: #eaf5ee !important;
    animation: support-pulse 2.2s ease-in-out infinite;
  }
  :global([data-theme="dark"] .bpmn-host .djs-element.support-active:not(.djs-connection) .djs-visual > :first-child) {
    stroke: #22a559 !important;
    fill: #0f2419 !important;
  }
  :global(.bpmn-host .djs-element.support-incident:not(.djs-connection) .djs-visual > :first-child) {
    stroke: #b91c1c !important;
    stroke-width: 3.5px !important;
    fill: #fdecec !important;
    animation: none;
  }
  :global([data-theme="dark"] .bpmn-host .djs-element.support-incident:not(.djs-connection) .djs-visual > :first-child) {
    fill: #3a1d1b !important;
  }
  :global(.bpmn-host .djs-element.support-selected .djs-visual > :first-child) {
    stroke: var(--accent) !important;
    stroke-width: 4px !important;
    stroke-dasharray: 6 4;
  }
  :global(.bpmn-host .support-count-badge) {
    min-width: 22px;
    height: 22px;
    padding: 0 6px;
    border-radius: 11px;
    background: #6b7572;
    border: 2px solid #ffffff;
    color: #fff;
    font: 700 10px var(--font-sans);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    pointer-events: none;
  }
  :global(.bpmn-host .bjs-breadcrumbs) {
    font-family: var(--font-sans);
    font-size: 12px;
    top: 10px;
    left: 10px;
    padding: 6px 10px;
    border-radius: 9px;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-1);
    color: var(--ink);
  }
  :global(.bpmn-host .bjs-breadcrumbs li a) {
    color: var(--accent);
  }
  :global(.bpmn-host .bjs-drilldown) {
    background: var(--accent);
    color: var(--on-accent);
    border-radius: 6px;
  }
  :global(.bpmn-host .bjs-powered-by) {
    display: none;
  }
  @keyframes support-pulse {
    0%, 100% { filter: drop-shadow(0 0 0 rgba(21, 128, 61, 0)); }
    50% { filter: drop-shadow(0 0 6px rgba(21, 128, 61, 0.55)); }
  }
</style>
