package com.malinatrash.camundasupport.ui.components

import java.security.MessageDigest
import java.util.LinkedHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import javafx.application.Platform
import javafx.concurrent.Worker
import javafx.scene.web.WebEngine
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import netscape.javascript.JSObject
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

internal object BpmnSvgRenderer {
    private const val MAX_CACHE_ENTRIES = 12
    private val renderMutex = Mutex()
    private val javaFxStarted = AtomicBoolean(false)
    private val cache = object : LinkedHashMap<String, RenderedBpmn>(MAX_CACHE_ENTRIES, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, RenderedBpmn>?): Boolean =
            size > MAX_CACHE_ENTRIES
    }

    suspend fun render(xml: String, diagramElementId: String? = null): RenderedBpmn {
        val key = withContext(Dispatchers.Default) {
            "${xml.sha256()}:${diagramElementId ?: ROOT_DIAGRAM_CACHE_KEY}"
        }
        synchronized(cache) { cache[key] }?.let { return it }
        return renderMutex.withLock {
            synchronized(cache) { cache[key] }?.let { return@withLock it }
            ensureJavaFxStarted()
            withTimeout(EXPORT_TIMEOUT_MILLIS) { exportSvg(xml, diagramElementId) }
                .also { rendered -> synchronized(cache) { cache[key] = rendered } }
        }
    }

    suspend fun renderAll(xml: String): RenderedBpmn {
        val root = render(xml)
        if (root.diagrams.size == 1) return root
        val renderedDiagrams = root.diagrams.toMutableMap()
        root.diagrams.keys
            .filterNot { it == root.rootElementId }
            .forEach { diagramElementId ->
                val rendered = render(xml, diagramElementId)
                renderedDiagrams[diagramElementId] = rendered.diagram(diagramElementId)
            }
        return root.copy(diagrams = renderedDiagrams)
    }

    private suspend fun ensureJavaFxStarted() = withContext(Dispatchers.IO) {
        if (javaFxStarted.get()) return@withContext
        synchronized(javaFxStarted) {
            if (javaFxStarted.get()) return@synchronized
            val ready = CountDownLatch(1)
            try {
                Platform.startup {
                    Platform.setImplicitExit(false)
                    ready.countDown()
                }
            } catch (_: IllegalStateException) {
                Platform.runLater {
                    Platform.setImplicitExit(false)
                    ready.countDown()
                }
            }
            check(ready.await(10, TimeUnit.SECONDS)) { "JavaFX toolkit не запустился за 10 секунд" }
            javaFxStarted.set(true)
        }
    }

    private suspend fun exportSvg(
        xml: String,
        diagramElementId: String?,
    ): RenderedBpmn = suspendCancellableCoroutine { continuation ->
        Platform.runLater {
            val engine = WebEngine().apply { isJavaScriptEnabled = true }
            val completed = AtomicBoolean(false)
            continuation.invokeOnCancellation {
                Platform.runLater {
                    completed.set(true)
                    engine.load("about:blank")
                }
            }
            fun finish(result: Result<RenderedBpmn>) {
                if (!completed.compareAndSet(false, true)) return
                engine.load("about:blank")
                if (!continuation.isActive) return
                result.fold(continuation::resume, continuation::resumeWithException)
            }
            val bridge = BpmnSvgExportBridge(
                onSuccess = { payload ->
                    finish(runCatching { parseRenderedBpmn(payload) })
                },
                onFailure = { message -> finish(Result.failure(IllegalStateException(message))) },
            )
            engine.loadWorker.stateProperty().addListener { _, _, state ->
                if (state == Worker.State.SUCCEEDED && !completed.get()) {
                    runCatching {
                        (engine.executeScript("window") as JSObject).setMember("supportExportBridge", bridge)
                        engine.executeScript("window.supportStartExport()")
                    }.onFailure { finish(Result.failure(it)) }
                } else if (state == Worker.State.FAILED && !completed.get()) {
                    finish(Result.failure(IllegalStateException("JavaFX WebView не загрузил модуль BPMN")))
                }
            }
            engine.loadContent(BpmnExportHtml.build(xml, diagramElementId), "text/html")
        }
    }

    private fun parseRenderedBpmn(payload: String): RenderedBpmn {
        val root = Json.parseToJsonElement(payload).jsonObject
        val diagrams = root.getValue("diagrams").jsonArray.map { value ->
            val diagram = value.jsonObject
            val elementId = diagram.getValue("elementId").jsonPrimitive.content
            val svg = diagram.getValue("svg").jsonPrimitive.content
            RenderedBpmnDiagram(
                elementId = elementId,
                name = diagram["name"]?.jsonPrimitive?.contentOrNull?.takeIf(String::isNotBlank) ?: elementId,
                parentElementId = diagram["parentElementId"]?.jsonPrimitive?.contentOrNull,
                svg = svg,
                elementIds = diagram["elementIds"]
                    ?.jsonArray
                    ?.mapNotNull { it.jsonPrimitive.contentOrNull }
                    ?.toSet()
                    .orEmpty()
                    .ifEmpty { SVG_ELEMENT_ID_REGEX.findAll(svg).map { it.groupValues[1] }.toSet() },
            )
        }.associateBy(RenderedBpmnDiagram::elementId)
        check(diagrams.isNotEmpty()) { "bpmn-js не вернул ни одной BPMN-диаграммы" }
        val rootElementId = root["rootElementId"]?.jsonPrimitive?.contentOrNull
            ?.takeIf(diagrams::containsKey)
            ?: diagrams.values.firstOrNull { it.parentElementId == null }?.elementId
            ?: diagrams.keys.first()
        return RenderedBpmn(rootElementId = rootElementId, diagrams = diagrams)
    }

    private fun String.sha256(): String = MessageDigest.getInstance("SHA-256")
        .digest(toByteArray())
        .joinToString("") { byte -> "%02x".format(byte) }

    private const val EXPORT_TIMEOUT_MILLIS = 15_000L
    private const val ROOT_DIAGRAM_CACHE_KEY = "__root__"
    private val SVG_ELEMENT_ID_REGEX = Regex("""data-element-id=["']([^"']+)["']""")
}

internal data class RenderedBpmn(
    val rootElementId: String,
    val diagrams: Map<String, RenderedBpmnDiagram>,
) {
    fun diagram(elementId: String?): RenderedBpmnDiagram =
        diagrams[elementId] ?: diagrams.getValue(rootElementId)

    fun childrenOf(elementId: String): List<RenderedBpmnDiagram> =
        diagrams.values.filter { it.parentElementId == elementId }

    fun containsInSubtree(diagramElementId: String, activityId: String): Boolean {
        val diagram = diagrams[diagramElementId] ?: return false
        if (activityId in diagram.elementIds) return true
        return childrenOf(diagramElementId).any { containsInSubtree(it.elementId, activityId) }
    }
}

internal data class RenderedBpmnDiagram(
    val elementId: String,
    val name: String,
    val parentElementId: String?,
    val svg: String,
    val elementIds: Set<String>,
)

class BpmnSvgExportBridge(
    private val onSuccess: (String) -> Unit,
    private val onFailure: (String) -> Unit,
) {
    @Suppress("unused")
    fun onSuccess(svg: String?) {
        if (svg.isNullOrBlank()) onFailure("bpmn-js вернул пустой SVG") else onSuccess.invoke(svg)
    }

    @Suppress("unused")
    fun onFailure(message: String?) {
        onFailure.invoke(message?.takeIf(String::isNotBlank) ?: "Не удалось подготовить BPMN-схему")
    }
}

internal object BpmnExportHtml {
    private val script by lazy { bpmnResourceText("bpmn-js/bpmn-navigated-viewer.production.min.js") }
    private val diagramCss by lazy { bpmnResourceText("bpmn-js/diagram-js.css") }
    private val bpmnCss by lazy { bpmnResourceText("bpmn-js/bpmn-js.css") }

    fun build(xml: String, diagramElementId: String? = null): String {
        val safeScript = script.replace("</script>", "<\\/script>")
        val xmlJson = JsonPrimitive(xml).toString()
        val requestedDiagramIdJson = JsonPrimitive(diagramElementId).toString()
        return """
            <!doctype html>
            <html lang="ru">
            <head>
              <meta charset="utf-8" />
              <style>
                $diagramCss
                $bpmnCss
                html, body, #canvas { width: 100%; height: 100%; margin: 0; overflow: hidden; }
                .djs-palette, .djs-context-pad, .bjs-powered-by { display: none !important; }
              </style>
            </head>
            <body>
              <div id="canvas"></div>
              <script>$safeScript</script>
              <script>
                const xml = $xmlJson;
                const requestedDiagramId = $requestedDiagramIdJson;
                let exportStarted = false;
                window.supportStartExport = () => {
                  if (exportStarted) return;
                  exportStarted = true;
                  const viewer = new BpmnJS({ container: '#canvas' });
                  const documentNode = new DOMParser().parseFromString(xml, 'application/xml');
                  const parserError = documentNode.querySelector('parsererror');
                  if (parserError) {
                    window.supportExportBridge.onFailure('Не удалось разобрать BPMN XML');
                    return;
                  }
                  const diagramNodes = Array.from(documentNode.getElementsByTagNameNS(
                    'http://www.omg.org/spec/BPMN/20100524/DI',
                    'BPMNDiagram'
                  ));
                  const elementById = new Map(
                    Array.from(documentNode.querySelectorAll('[id]'))
                      .map(element => [String(element.getAttribute('id')), element])
                  );
                  const rootIds = new Set(
                    diagramNodes
                      .map(diagram => diagram.querySelector('[bpmnElement]'))
                      .filter(Boolean)
                      .map(plane => String(plane.getAttribute('bpmnElement')))
                  );
                  const diagrams = diagramNodes.map(diagram => {
                    const plane = diagram.querySelector('[bpmnElement]');
                    if (!plane) return null;
                    const elementId = String(plane.getAttribute('bpmnElement') || '');
                    const element = elementById.get(elementId);
                    if (!elementId || !element) return null;
                    let parent = element.parentElement;
                    while (parent && !rootIds.has(String(parent.getAttribute('id') || ''))) {
                      parent = parent.parentElement;
                    }
                    return {
                      elementId,
                      name: String(element.getAttribute('name') || elementId),
                      parentElementId: parent ? String(parent.getAttribute('id')) : null,
                      elementIds: Array.from(plane.querySelectorAll('[bpmnElement]'))
                        .map(item => String(item.getAttribute('bpmnElement') || ''))
                        .filter(Boolean),
                      sourceNode: diagram
                    };
                  }).filter(Boolean);
                  if (!diagrams.length) {
                    window.supportExportBridge.onFailure('В BPMN отсутствуют BPMNDiagram');
                    return;
                  }
                  const rootDiagram = diagrams.find(diagram => !diagram.parentElementId) || diagrams[0];
                  const selectedDiagram = diagrams.find(diagram => diagram.elementId === requestedDiagramId)
                    || rootDiagram;
                  diagrams.forEach(diagram => {
                    if (diagram !== selectedDiagram) diagram.sourceNode.remove();
                  });
                  const selectedXml = new XMLSerializer().serializeToString(documentNode);
                  viewer.importXML(selectedXml)
                    .then(async () => {
                      const exported = await viewer.saveSVG();
                      if (!exported.svg) {
                        throw new Error('bpmn-js вернул пустой SVG для ' + selectedDiagram.elementId);
                      }
                      diagrams.forEach(diagram => {
                        delete diagram.sourceNode;
                        diagram.svg = diagram.elementId === selectedDiagram.elementId
                          ? String(exported.svg)
                          : '';
                      });
                      const payload = JSON.stringify({
                        rootElementId: rootDiagram.elementId,
                        diagrams
                      });
                      viewer.destroy();
                      document.getElementById('canvas').replaceChildren();
                      window.supportExportBridge.onSuccess(payload);
                    })
                    .catch(error => {
                      viewer.destroy();
                      window.supportExportBridge.onFailure(String(error.message || error));
                    });
                };
              </script>
            </body>
            </html>
        """.trimIndent()
    }
}

internal fun bpmnResourceText(path: String): String = requireNotNull(
    Thread.currentThread().contextClassLoader.getResourceAsStream(path),
) { "Ресурс $path не найден" }.bufferedReader().use { it.readText() }
