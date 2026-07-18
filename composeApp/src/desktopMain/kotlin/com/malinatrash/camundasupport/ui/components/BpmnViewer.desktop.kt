package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.awt.SwingPanel
import androidx.compose.ui.graphics.Color
import javafx.application.Platform
import javafx.embed.swing.JFXPanel
import javafx.scene.Scene
import javafx.scene.input.ScrollEvent
import javafx.scene.input.ZoomEvent
import javafx.scene.web.WebEngine
import javafx.scene.web.WebView
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonPrimitive
import netscape.javascript.JSObject
import javax.swing.SwingUtilities

@Composable
actual fun BpmnViewer(
    xml: String,
    activeActivityIds: Set<String>,
    incidentActivityIds: Set<String>,
    clickableActivityIds: Set<String>,
    onActivityClick: (String) -> Unit,
    modifier: Modifier,
) {
    val html = remember(xml, activeActivityIds, incidentActivityIds, clickableActivityIds) {
        BpmnHtml.build(xml, activeActivityIds, incidentActivityIds, clickableActivityIds)
    }
    Box(modifier.background(Color(0xFFF8FAFC)), contentAlignment = Alignment.Center) {
        SwingPanel(
            modifier = Modifier.fillMaxSize(),
            factory = { BpmnWebViewPanel() },
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
                zoomDiagram(event.zoomFactor)
                event.consume()
            }
            browser.addEventFilter(ScrollEvent.SCROLL) { event ->
                if (event.isControlDown || event.isMetaDown) {
                    zoomDiagram(if (event.deltaY > 0) 1.15 else 1.0 / 1.15)
                    event.consume()
                }
            }
            scene = Scene(browser)
            loadRequestedHtml()
        }
    }

    fun render(html: String) {
        if (requestedHtml == html) return
        requestedHtml = html
        Platform.runLater(::loadRequestedHtml)
    }

    private fun loadRequestedHtml() {
        val html = requestedHtml
        val currentEngine = engine ?: return
        if (html.isBlank() || renderedHtml == html) return
        renderedHtml = html
        currentEngine.loadContent(html, "text/html")
    }

    private fun zoomDiagram(factor: Double) {
        runCatching {
            engine?.executeScript("window.supportZoomBy && window.supportZoomBy($factor)")
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
    private val script by lazy { resourceText("bpmn-js/bpmn-navigated-viewer.production.min.js") }
    private val diagramCss by lazy { resourceText("bpmn-js/diagram-js.css") }
    private val bpmnCss by lazy { resourceText("bpmn-js/bpmn-js.css") }

    fun build(
        xml: String,
        activeIds: Set<String>,
        incidentIds: Set<String>,
        clickableIds: Set<String>,
    ): String {
        val safeScript = script.replace("</script>", "<\\/script>")
        val xmlJson = JsonPrimitive(xml).toString()
        val activeJson = JsonArray(activeIds.sorted().map(::JsonPrimitive)).toString()
        val incidentJson = JsonArray(incidentIds.sorted().map(::JsonPrimitive)).toString()
        val clickableJson = JsonArray(clickableIds.sorted().map(::JsonPrimitive)).toString()
        return """
            <!doctype html>
            <html lang="ru">
            <head>
              <meta charset="utf-8" />
              <meta name="viewport" content="width=device-width, initial-scale=1" />
              <style>
                $diagramCss
                $bpmnCss
                html, body, #canvas { width: 100%; height: 100%; margin: 0; overflow: hidden; background: #f8fafc; }
                body { font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
                .djs-container { background: #f8fafc; }
                .djs-palette, .djs-context-pad, .bjs-powered-by { display: none !important; }
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
                .support-active:not(.djs-connection) .djs-visual > :first-child {
                  stroke: #3157d5 !important; stroke-width: 4px !important; fill: #e8edff !important;
                }
                .support-active.djs-connection .djs-visual > path { stroke: #3157d5 !important; stroke-width: 4px !important; }
                .support-incident:not(.djs-connection) .djs-visual > :first-child {
                  stroke: #d92d20 !important; stroke-width: 4px !important; fill: #fff0ee !important;
                }
                .support-clickable { cursor: pointer !important; }
                .support-clickable:hover:not(.djs-connection) .djs-visual > :first-child {
                  stroke: #3157d5 !important; stroke-width: 3px !important;
                  filter: drop-shadow(0 3px 6px rgba(49,87,213,.22));
                }
                .support-selected:not(.djs-connection) .djs-visual > :first-child {
                  stroke: #3157d5 !important; stroke-width: 5px !important; fill: #eef2ff !important;
                }
              </style>
            </head>
            <body>
              <div id="canvas"></div>
              <div class="support-zoom-controls" aria-label="Управление масштабом">
                <button id="zoom-out" type="button" title="Уменьшить масштаб">−</button>
                <button id="zoom-fit" type="button" title="Показать схему целиком">По размеру</button>
                <button id="zoom-in" type="button" title="Увеличить масштаб">+</button>
              </div>
              <script>$safeScript</script>
              <script>
                const viewer = new BpmnJS({ container: '#canvas' });
                const xml = $xmlJson;
                const activeIds = $activeJson;
                const incidentIds = $incidentJson;
                const clickableIds = $clickableJson;
                let supportCanvas = null;
                let selectedActivityId = null;
                window.supportZoomBy = factor => {
                  if (!supportCanvas) return;
                  const current = supportCanvas.zoom();
                  const next = Math.max(0.2, Math.min(4, current * factor));
                  supportCanvas.zoom(next);
                };
                document.getElementById('zoom-out').addEventListener('click', () => window.supportZoomBy(1 / 1.2));
                document.getElementById('zoom-in').addEventListener('click', () => window.supportZoomBy(1.2));
                document.getElementById('zoom-fit').addEventListener('click', () => {
                  if (supportCanvas) supportCanvas.zoom('fit-viewport');
                });
                document.addEventListener('wheel', event => {
                  if (!event.ctrlKey && !event.metaKey) return;
                  event.preventDefault();
                  event.stopImmediatePropagation();
                  window.supportZoomBy(event.deltaY < 0 ? 1.15 : 1 / 1.15);
                }, { passive: false, capture: true });
                viewer.importXML(xml).then(() => {
                  const canvas = viewer.get('canvas');
                  supportCanvas = canvas;
                  const registry = viewer.get('elementRegistry');
                  activeIds.forEach(id => { if (registry.get(id)) canvas.addMarker(id, 'support-active'); });
                  incidentIds.forEach(id => { if (registry.get(id)) canvas.addMarker(id, 'support-incident'); });
                  clickableIds.forEach(id => { if (registry.get(id)) canvas.addMarker(id, 'support-clickable'); });
                  window.supportSelectActivity = id => {
                    if (!id || !clickableIds.includes(id)) return;
                    if (selectedActivityId && registry.get(selectedActivityId)) {
                      canvas.removeMarker(selectedActivityId, 'support-selected');
                    }
                    selectedActivityId = id;
                    canvas.addMarker(id, 'support-selected');
                    if (window.supportBridge && window.supportBridge.onActivityClick) {
                      window.supportBridge.onActivityClick(String(id));
                    }
                  };
                  const eventBus = viewer.get('eventBus');
                  eventBus.on('element.click', event => {
                    const id = event && event.element && event.element.id;
                    window.supportSelectActivity(id);
                  });
                  canvas.zoom('fit-viewport');
                }).catch(error => {
                  document.body.innerHTML = '<div style="padding:24px;color:#b42318">Не удалось отрисовать BPMN: ' +
                    String(error.message || error) + '</div>';
                });
              </script>
            </body>
            </html>
        """.trimIndent()
    }

    private fun resourceText(path: String): String = requireNotNull(
        Thread.currentThread().contextClassLoader.getResourceAsStream(path),
    ) { "Ресурс $path не найден" }.bufferedReader().use { it.readText() }
}
