package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.awt.SwingPanel
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.sp
import javafx.application.Platform
import javafx.embed.swing.JFXPanel
import javafx.scene.Scene
import javafx.scene.input.ScrollEvent
import javafx.scene.input.ZoomEvent
import javafx.scene.web.WebEngine
import javafx.scene.web.WebView
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import netscape.javascript.JSObject
import java.util.concurrent.atomic.AtomicReference
import javax.swing.SwingUtilities

internal const val BPMN_SHARP_RENDER_DEBOUNCE_MILLIS = 2_000

@Composable
actual fun BpmnViewer(
    xml: String,
    activeActivityIds: Set<String>,
    incidentActivityIds: Set<String>,
    completedActivityCounts: Map<String, Int>,
    clickableActivityIds: Set<String>,
    onActivityClick: (String) -> Unit,
    modifier: Modifier,
) {
    val svgResult by produceState<Result<String>?>(initialValue = null, xml) {
        value = null
        value = runCatching { BpmnSvgRenderer.render(xml) }
    }
    val svg = svgResult?.getOrNull()
    if (svg == null) {
        Column(
            modifier = modifier.background(Color(0xFFF8FAFC)),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            if (svgResult == null) {
                CircularProgressIndicator()
                Text("Подготавливаем большую BPMN-схему…", color = Color(0xFF667085), fontSize = 11.sp)
            } else {
                Text(
                    "Не удалось подготовить BPMN: ${svgResult?.exceptionOrNull()?.message.orEmpty()}",
                    color = Color(0xFFB42318),
                    fontSize = 12.sp,
                )
            }
        }
        return
    }
    BpmnSvgPanel(
        svg = svg,
        activeActivityIds = activeActivityIds,
        incidentActivityIds = incidentActivityIds,
        completedActivityCounts = completedActivityCounts,
        clickableActivityIds = clickableActivityIds,
        onActivityClick = onActivityClick,
        modifier = modifier,
    )
}

@Composable
private fun BpmnSvgPanel(
    svg: String,
    activeActivityIds: Set<String>,
    incidentActivityIds: Set<String>,
    completedActivityCounts: Map<String, Int>,
    clickableActivityIds: Set<String>,
    onActivityClick: (String) -> Unit,
    modifier: Modifier,
) {
    val html = remember(svg, activeActivityIds, incidentActivityIds, completedActivityCounts, clickableActivityIds) {
        BpmnHtml.build(svg, activeActivityIds, incidentActivityIds, completedActivityCounts, clickableActivityIds)
    }
    val panelReference = remember { AtomicReference<BpmnWebViewPanel?>() }
    DisposableEffect(panelReference) {
        onDispose { panelReference.getAndSet(null)?.dispose() }
    }
    Box(modifier.background(Color(0xFFF8FAFC)), contentAlignment = Alignment.Center) {
        SwingPanel(
            modifier = Modifier.fillMaxSize(),
            factory = { BpmnWebViewPanel().also(panelReference::set) },
            update = {
                it.onActivityClick = onActivityClick
                it.render(html)
            },
        )
    }
}

private class BpmnWebViewPanel : JFXPanel() {
    @Volatile
    private var requestedHtml = ""
    private var renderedHtml = ""
    private var engine: WebEngine? = null
    @Volatile
    private var disposed = false
    private val bridge = BpmnWebBridge { activityId ->
        SwingUtilities.invokeLater { onActivityClick(activityId) }
    }

    @Volatile
    var onActivityClick: (String) -> Unit = {}

    init {
        Platform.setImplicitExit(false)
        Platform.runLater {
            val browser = WebView()
            engine = browser.engine.apply {
                isJavaScriptEnabled = true
                loadWorker.stateProperty().addListener { _, _, state ->
                    if (state == javafx.concurrent.Worker.State.SUCCEEDED) {
                        runCatching {
                            (executeScript("window") as JSObject).setMember("supportBridge", bridge)
                        }
                    }
                }
            }
            browser.addEventFilter(ZoomEvent.ZOOM) { event ->
                queueDiagramZoom(event.zoomFactor)
                event.consume()
            }
            browser.addEventFilter(ScrollEvent.SCROLL) { event ->
                if (event.isControlDown || event.isMetaDown) {
                    queueDiagramZoom(if (event.deltaY > 0) 1.15 else 1.0 / 1.15)
                    event.consume()
                }
            }
            scene = Scene(browser)
            loadRequestedHtml()
        }
    }

    fun render(html: String) {
        if (disposed || requestedHtml == html) return
        requestedHtml = html
        Platform.runLater(::loadRequestedHtml)
    }

    fun dispose() {
        disposed = true
        requestedHtml = ""
        Platform.runLater {
            engine?.load(null)
            engine = null
            scene = null
        }
    }

    private fun loadRequestedHtml() {
        if (disposed) return
        val html = requestedHtml
        val currentEngine = engine ?: return
        if (html.isBlank() || renderedHtml == html) return
        renderedHtml = html
        currentEngine.loadContent(html, "text/html")
    }

    private fun queueDiagramZoom(factor: Double) {
        runCatching {
            engine?.executeScript("window.supportQueueZoom && window.supportQueueZoom($factor)")
        }
    }
}

class BpmnWebBridge(private val onClick: (String) -> Unit) {
    @Suppress("unused")
    fun onActivityClick(activityId: String?) {
        activityId?.takeIf(String::isNotBlank)?.let(onClick)
    }
}

internal object BpmnHtml {
    private val diagramCss by lazy { bpmnResourceText("bpmn-js/diagram-js.css") }
    private val bpmnCss by lazy { bpmnResourceText("bpmn-js/bpmn-js.css") }

    fun build(
        svg: String,
        activeIds: Set<String>,
        incidentIds: Set<String>,
        completedCounts: Map<String, Int>,
        clickableIds: Set<String>,
    ): String {
        val svgJson = JsonPrimitive(svg).toString()
        val activeJson = JsonArray(activeIds.sorted().map(::JsonPrimitive)).toString()
        val incidentJson = JsonArray(incidentIds.sorted().map(::JsonPrimitive)).toString()
        val completedJson = buildJsonObject {
            completedCounts.toSortedMap().forEach { (activityId, count) -> put(activityId, count) }
        }.toString()
        val clickableJson = JsonArray(clickableIds.sorted().map(::JsonPrimitive)).toString()
        val embeddedSvgCssJson = JsonPrimitive(
            """
                $diagramCss
                $bpmnCss
                .support-completed:not(.djs-connection) .djs-visual > :first-child {
                  stroke: #079455 !important; stroke-width: 3px !important; fill: #ecfdf3 !important;
                }
                .support-completed.djs-connection .djs-visual > path { stroke: #079455 !important; stroke-width: 3px !important; }
                .support-active:not(.djs-connection) .djs-visual > :first-child {
                  stroke: #3157d5 !important; stroke-width: 4px !important; fill: #e8edff !important;
                }
                .support-active.djs-connection .djs-visual > path { stroke: #3157d5 !important; stroke-width: 4px !important; }
                .support-incident:not(.djs-connection) .djs-visual > :first-child {
                  stroke: #d92d20 !important; stroke-width: 4px !important; fill: #fff0ee !important;
                }
                .support-count-badge { pointer-events: none; }
                .support-count-badge circle { fill: #079455; stroke: #ffffff; stroke-width: 2px; }
                .support-count-badge text {
                  fill: #ffffff; font: 700 10px Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
                  text-anchor: middle; dominant-baseline: central;
                }
            """.trimIndent(),
        ).toString()
        return """
            <!doctype html>
            <html lang="ru">
            <head>
              <meta charset="utf-8" />
              <meta name="viewport" content="width=device-width, initial-scale=1" />
              <style>
                html, body, #canvas { width: 100%; height: 100%; margin: 0; overflow: hidden; background: #f8fafc; }
                body { font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
                #canvas { position: relative; cursor: grab; contain: strict; user-select: none; }
                #canvas.support-dragging { cursor: grabbing; }
                #diagram-stage {
                  position: absolute; left: 0; top: 0; transform-origin: 0 0;
                  will-change: transform; backface-visibility: hidden; contain: strict;
                }
                #diagram-bitmap { position: absolute; inset: 0; width: 100%; height: 100%; }
                #diagram-hits { position: absolute; inset: 0; pointer-events: none; }
                #sharp-viewport {
                  position: absolute; inset: 0; z-index: 2; overflow: hidden;
                  pointer-events: none; opacity: 0; transition: opacity 80ms linear;
                  contain: strict;
                }
                #sharp-viewport img { width: 100%; height: 100%; display: block; }
                .support-hit {
                  position: absolute; box-sizing: border-box; pointer-events: auto; cursor: pointer;
                  border: 2px solid transparent; border-radius: 5px; background: transparent;
                }
                .support-hit:hover {
                  border-color: rgba(49,87,213,.8); background: rgba(49,87,213,.10);
                  box-shadow: 0 0 0 2px rgba(49,87,213,.12);
                }
                .support-hit.support-selected {
                  border-color: #3157d5; background: rgba(49,87,213,.14);
                  box-shadow: 0 0 0 2px rgba(49,87,213,.18);
                }
                #source-host {
                  position: absolute; left: 0; top: 0; visibility: hidden;
                  pointer-events: none; overflow: visible;
                }
                .support-zoom-controls {
                  position: fixed; top: 12px; right: 12px; z-index: 1000;
                  display: flex; align-items: center; gap: 4px; padding: 4px;
                  border: 1px solid #d0d5dd; border-radius: 9px; background: rgba(255,255,255,.96);
                  box-shadow: 0 2px 8px rgba(16,24,40,.10);
                }
                .support-zoom-controls button {
                  height: 32px; min-width: 34px; padding: 0 10px; border: 1px solid transparent;
                  border-radius: 6px; background: transparent; color: #344054; cursor: pointer;
                  font: 600 13px Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
                }
                .support-zoom-controls button:hover { background: #f2f4f7; border-color: #e4e7ec; }
                .support-zoom-controls button:active { background: #e8edff; color: #3157d5; }
              </style>
            </head>
            <body>
              <div id="canvas"></div>
              <div id="sharp-viewport" aria-hidden="true"></div>
              <div class="support-zoom-controls" aria-label="Управление масштабом">
                <button id="zoom-out" type="button" title="Уменьшить масштаб">−</button>
                <button id="zoom-fit" type="button" title="Показать схему целиком">По размеру</button>
                <button id="zoom-in" type="button" title="Увеличить масштаб">+</button>
              </div>
              <script>
                const canvas = document.getElementById('canvas');
                const sharpViewport = document.getElementById('sharp-viewport');
                const sourceHost = document.createElement('div');
                sourceHost.id = 'source-host';
                sourceHost.innerHTML = $svgJson;
                canvas.appendChild(sourceHost);
                const svg = sourceHost.querySelector('svg');
                const activeIds = $activeJson;
                const incidentIds = $incidentJson;
                const completedCounts = $completedJson;
                const clickableIds = $clickableJson;
                const embeddedSvgCss = $embeddedSvgCssJson;
                const elementById = new Map();
                const hitById = new Map();
                window.supportDiagramReady = false;
                window.supportSharpReady = false;
                let selectedActivityId = null;
                let interactionFrame = 0;
                let pendingPanX = 0;
                let pendingPanY = 0;
                let pendingZoomFactor = 1;
                let stage = null;
                let stageScale = 1;
                let stageX = 0;
                let stageY = 0;
                let fitScale = 1;
                let stageWidth = 1;
                let stageHeight = 1;
                let diagramBounds = null;
                let serializedSvg = null;
                let sharpRenderTimer = 0;
                let sharpRenderToken = 0;
                let sharpImageUrl = null;
                const hideSharpViewport = () => {
                  sharpRenderToken += 1;
                  if (sharpRenderTimer) clearTimeout(sharpRenderTimer);
                  sharpRenderTimer = 0;
                  sharpViewport.style.opacity = '0';
                  window.supportSharpReady = false;
                };
                const renderSharpViewport = () => {
                  sharpRenderTimer = 0;
                  if (!stage || !serializedSvg || !window.supportDiagramReady) return;
                  const viewportWidth = Math.max(1, canvas.clientWidth);
                  const viewportHeight = Math.max(1, canvas.clientHeight);
                  const visibleStageX = -stageX / stageScale;
                  const visibleStageY = -stageY / stageScale;
                  const visibleStageWidth = viewportWidth / stageScale;
                  const visibleStageHeight = viewportHeight / stageScale;
                  const viewX = diagramBounds.x + visibleStageX * diagramBounds.width / stageWidth;
                  const viewY = diagramBounds.y + visibleStageY * diagramBounds.height / stageHeight;
                  const viewWidth = visibleStageWidth * diagramBounds.width / stageWidth;
                  const viewHeight = visibleStageHeight * diagramBounds.height / stageHeight;
                  const pixelRatio = Math.min(2, Math.max(1, window.devicePixelRatio || 1));
                  const sharpSvg = svg;
                  sharpSvg.setAttribute('width', String(Math.round(viewportWidth * pixelRatio)));
                  sharpSvg.setAttribute('height', String(Math.round(viewportHeight * pixelRatio)));
                  sharpSvg.setAttribute('viewBox', [viewX, viewY, viewWidth, viewHeight].join(' '));
                  sharpSvg.setAttribute('preserveAspectRatio', 'none');
                  const markup = new XMLSerializer().serializeToString(sharpSvg);
                  const url = URL.createObjectURL(new Blob([markup], { type: 'image/svg+xml' }));
                  const token = ++sharpRenderToken;
                  const image = new Image();
                  image.onload = () => {
                    if (token !== sharpRenderToken) {
                      URL.revokeObjectURL(url);
                      return;
                    }
                    const previousUrl = sharpImageUrl;
                    sharpImageUrl = url;
                    sharpViewport.replaceChildren(image);
                    sharpViewport.style.opacity = '1';
                    window.supportSharpReady = true;
                    if (previousUrl) URL.revokeObjectURL(previousUrl);
                  };
                  image.onerror = () => URL.revokeObjectURL(url);
                  image.src = url;
                };
                const scheduleSharpViewport = () => {
                  if (sharpRenderTimer) clearTimeout(sharpRenderTimer);
                  sharpRenderTimer = setTimeout(renderSharpViewport, $BPMN_SHARP_RENDER_DEBOUNCE_MILLIS);
                };
                const applyTransform = () => {
                  if (!stage) return;
                  stage.style.transform = 'translate3d(' + stageX + 'px,' + stageY + 'px,0) scale(' + stageScale + ')';
                  scheduleSharpViewport();
                };
                const fitDiagram = () => {
                  if (!stage) return;
                  fitScale = Math.max(0.0001, Math.min(
                    canvas.clientWidth / stageWidth,
                    canvas.clientHeight / stageHeight
                  ) * 0.96);
                  stageScale = fitScale;
                  stageX = (canvas.clientWidth - stageWidth * stageScale) / 2;
                  stageY = (canvas.clientHeight - stageHeight * stageScale) / 2;
                  applyTransform();
                };
                const flushInteraction = () => {
                  interactionFrame = 0;
                  if (!stage) return;
                  if (pendingPanX || pendingPanY) {
                    stageX -= pendingPanX;
                    stageY -= pendingPanY;
                    pendingPanX = 0;
                    pendingPanY = 0;
                  }
                  if (pendingZoomFactor !== 1) {
                    const factor = Math.max(0.25, Math.min(4, pendingZoomFactor));
                    const oldScale = stageScale;
                    const maxScale = Math.max(2, fitScale * 16);
                    const newScale = Math.max(fitScale * 0.25, Math.min(maxScale, oldScale * factor));
                    const centerX = canvas.clientWidth / 2;
                    const centerY = canvas.clientHeight / 2;
                    const ratio = newScale / oldScale;
                    stageX = centerX - (centerX - stageX) * ratio;
                    stageY = centerY - (centerY - stageY) * ratio;
                    stageScale = newScale;
                    pendingZoomFactor = 1;
                  }
                  applyTransform();
                };
                const scheduleInteraction = () => {
                  if (!interactionFrame) interactionFrame = requestAnimationFrame(flushInteraction);
                };
                window.supportQueueZoom = factor => {
                  hideSharpViewport();
                  pendingZoomFactor *= factor;
                  scheduleInteraction();
                };
                window.supportQueuePan = (deltaX, deltaY) => {
                  hideSharpViewport();
                  pendingPanX += deltaX;
                  pendingPanY += deltaY;
                  scheduleInteraction();
                };
                document.getElementById('zoom-out').addEventListener('click', () => window.supportQueueZoom(1 / 1.2));
                document.getElementById('zoom-in').addEventListener('click', () => window.supportQueueZoom(1.2));
                document.getElementById('zoom-fit').addEventListener('click', fitDiagram);
                document.addEventListener('wheel', event => {
                  event.preventDefault();
                  event.stopImmediatePropagation();
                  if (event.ctrlKey || event.metaKey) {
                    window.supportQueueZoom(Math.exp(-event.deltaY * 0.002));
                  } else {
                    window.supportQueuePan(event.deltaX, event.deltaY);
                  }
                }, { passive: false, capture: true });
                if (!svg) {
                  document.body.innerHTML = '<div style="padding:24px;color:#b42318">Не удалось открыть подготовленный SVG</div>';
                } else {
                  const rawViewBox = svg.getAttribute('viewBox');
                  if (rawViewBox) {
                    const values = rawViewBox.trim().split(/[ ,]+/).map(Number);
                    diagramBounds = { x: values[0], y: values[1], width: values[2], height: values[3] };
                  } else {
                    const bounds = svg.getBBox();
                    diagramBounds = { x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height };
                  }
                  const sourceWidth = Math.max(1, diagramBounds.width);
                  const sourceHeight = Math.max(1, diagramBounds.height);
                  const bitmapScale = Math.min(
                    1,
                    6144 / sourceWidth,
                    4096 / sourceHeight,
                    Math.sqrt(16000000 / (sourceWidth * sourceHeight))
                  );
                  stageWidth = Math.max(1, Math.round(sourceWidth * bitmapScale));
                  stageHeight = Math.max(1, Math.round(sourceHeight * bitmapScale));
                  sourceHost.style.width = stageWidth + 'px';
                  sourceHost.style.height = stageHeight + 'px';
                  svg.setAttribute('width', String(stageWidth));
                  svg.setAttribute('height', String(stageHeight));
                  svg.setAttribute('viewBox', [diagramBounds.x, diagramBounds.y, diagramBounds.width, diagramBounds.height].join(' '));
                  svg.setAttribute('preserveAspectRatio', 'none');
                  canvas.querySelectorAll('[data-element-id]').forEach(element => {
                    elementById.set(element.getAttribute('data-element-id'), element);
                  });
                  Object.entries(completedCounts).forEach(([id, count]) => {
                    const element = elementById.get(id);
                    if (!element || count < 1) return;
                    element.classList.add('support-completed');
                    const visual = element.querySelector('.djs-visual');
                    if (!visual) return;
                    const bounds = visual.getBBox();
                    const svgNs = 'http://www.w3.org/2000/svg';
                    const badge = document.createElementNS(svgNs, 'g');
                    badge.setAttribute('class', 'support-count-badge');
                    badge.setAttribute('transform', 'translate(' + (bounds.x + bounds.width - 3) + ', ' + (bounds.y + 3) + ')');
                    const circle = document.createElementNS(svgNs, 'circle');
                    circle.setAttribute('r', '12');
                    const label = document.createElementNS(svgNs, 'text');
                    label.textContent = '×' + count;
                    badge.appendChild(circle);
                    badge.appendChild(label);
                    element.appendChild(badge);
                  });
                  activeIds.forEach(id => elementById.get(id)?.classList.add('support-active'));
                  incidentIds.forEach(id => elementById.get(id)?.classList.add('support-incident'));

                  const svgNs = 'http://www.w3.org/2000/svg';
                  const embeddedStyle = document.createElementNS(svgNs, 'style');
                  embeddedStyle.textContent = embeddedSvgCss;
                  svg.insertBefore(embeddedStyle, svg.firstChild);

                  stage = document.createElement('div');
                  stage.id = 'diagram-stage';
                  stage.style.width = stageWidth + 'px';
                  stage.style.height = stageHeight + 'px';
                  const bitmap = document.createElement('canvas');
                  bitmap.id = 'diagram-bitmap';
                  bitmap.width = stageWidth;
                  bitmap.height = stageHeight;
                  const hitLayer = document.createElement('div');
                  hitLayer.id = 'diagram-hits';
                  stage.appendChild(bitmap);
                  stage.appendChild(hitLayer);
                  canvas.appendChild(stage);

                  const svgRect = svg.getBoundingClientRect();
                  clickableIds.forEach(id => {
                    const element = elementById.get(id);
                    const visual = element?.querySelector('.djs-visual') || element;
                    if (!visual) return;
                    const rect = visual.getBoundingClientRect();
                    if (rect.width <= 0 || rect.height <= 0) return;
                    const hit = document.createElement('div');
                    hit.className = 'support-hit';
                    hit.dataset.activityId = id;
                    hit.style.left = (rect.left - svgRect.left) + 'px';
                    hit.style.top = (rect.top - svgRect.top) + 'px';
                    hit.style.width = rect.width + 'px';
                    hit.style.height = rect.height + 'px';
                    hitById.set(id, hit);
                    hitLayer.appendChild(hit);
                  });
                  window.supportSelectActivity = id => {
                    if (!id || !clickableIds.includes(id)) return;
                    if (selectedActivityId) hitById.get(selectedActivityId)?.classList.remove('support-selected');
                    selectedActivityId = id;
                    hitById.get(id)?.classList.add('support-selected');
                    if (window.supportBridge && window.supportBridge.onActivityClick) {
                      window.supportBridge.onActivityClick(String(id));
                    }
                  };
                  clickableIds.forEach(id => {
                    const hit = hitById.get(id);
                    if (!hit) return;
                    hit.addEventListener('click', event => {
                      event.stopPropagation();
                      window.supportSelectActivity(id);
                    });
                  });

                  serializedSvg = new XMLSerializer().serializeToString(svg);
                  const imageUrl = URL.createObjectURL(new Blob([serializedSvg], { type: 'image/svg+xml' }));
                  const image = new Image();
                  image.onload = () => {
                    const context = bitmap.getContext('2d', { alpha: false });
                    context.fillStyle = '#f8fafc';
                    context.fillRect(0, 0, stageWidth, stageHeight);
                    context.drawImage(image, 0, 0, stageWidth, stageHeight);
                    URL.revokeObjectURL(imageUrl);
                    sourceHost.remove();
                    window.supportDiagramReady = true;
                    scheduleSharpViewport();
                  };
                  image.onerror = () => {
                    URL.revokeObjectURL(imageUrl);
                    sourceHost.style.visibility = 'visible';
                    window.supportDiagramError = 'Не удалось растрировать BPMN';
                  };
                  image.src = imageUrl;

                  let dragging = false;
                  let lastX = 0;
                  let lastY = 0;
                  canvas.addEventListener('mousedown', event => {
                    if (event.button !== 0) return;
                    dragging = true;
                    canvas.classList.add('support-dragging');
                    lastX = event.clientX;
                    lastY = event.clientY;
                  });
                  window.addEventListener('mousemove', event => {
                    if (!dragging) return;
                    window.supportQueuePan(lastX - event.clientX, lastY - event.clientY);
                    lastX = event.clientX;
                    lastY = event.clientY;
                  });
                  window.addEventListener('mouseup', () => {
                    dragging = false;
                    canvas.classList.remove('support-dragging');
                  });
                  window.addEventListener('resize', fitDiagram);
                  window.addEventListener('beforeunload', () => {
                    if (sharpImageUrl) URL.revokeObjectURL(sharpImageUrl);
                  });
                  fitDiagram();
                }
              </script>
            </body>
            </html>
        """.trimIndent()
    }
}
