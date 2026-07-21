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
import kotlinx.serialization.json.JsonPrimitive
import netscape.javascript.JSObject
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

internal object BpmnSvgRenderer {
    private const val MAX_CACHE_ENTRIES = 4
    private val renderMutex = Mutex()
    private val javaFxStarted = AtomicBoolean(false)
    private val cache = object : LinkedHashMap<String, String>(MAX_CACHE_ENTRIES, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, String>?): Boolean =
            size > MAX_CACHE_ENTRIES
    }

    suspend fun render(xml: String): String {
        val key = withContext(Dispatchers.Default) { xml.sha256() }
        synchronized(cache) { cache[key] }?.let { return it }
        return renderMutex.withLock {
            synchronized(cache) { cache[key] }?.let { return@withLock it }
            ensureJavaFxStarted()
            withTimeout(EXPORT_TIMEOUT_MILLIS) { exportSvg(xml) }
                .also { svg -> synchronized(cache) { cache[key] = svg } }
        }
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

    private suspend fun exportSvg(xml: String): String = suspendCancellableCoroutine { continuation ->
        Platform.runLater {
            val engine = WebEngine().apply { isJavaScriptEnabled = true }
            val completed = AtomicBoolean(false)
            continuation.invokeOnCancellation {
                Platform.runLater {
                    completed.set(true)
                    engine.load(null)
                }
            }
            fun finish(result: Result<String>) {
                if (!completed.compareAndSet(false, true)) return
                engine.load(null)
                if (!continuation.isActive) return
                result.fold(continuation::resume, continuation::resumeWithException)
            }
            val bridge = BpmnSvgExportBridge(
                onSuccess = { svg -> finish(Result.success(svg)) },
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
            engine.loadContent(BpmnExportHtml.build(xml), "text/html")
        }
    }

    private fun String.sha256(): String = MessageDigest.getInstance("SHA-256")
        .digest(toByteArray())
        .joinToString("") { byte -> "%02x".format(byte) }

    private const val EXPORT_TIMEOUT_MILLIS = 30_000L
}

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

    fun build(xml: String): String {
        val safeScript = script.replace("</script>", "<\\/script>")
        val xmlJson = JsonPrimitive(xml).toString()
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
                let exportStarted = false;
                window.supportStartExport = () => {
                  if (exportStarted) return;
                  exportStarted = true;
                  const viewer = new BpmnJS({ container: '#canvas' });
                  viewer.importXML(xml)
                    .then(() => {
                      const svg = document.querySelector('#canvas svg');
                      if (!svg) throw new Error('bpmn-js не создал SVG');
                      window.supportExportBridge.onSuccess(String(svg.outerHTML || ''));
                    })
                    .catch(error => window.supportExportBridge.onFailure(String(error.message || error)));
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
