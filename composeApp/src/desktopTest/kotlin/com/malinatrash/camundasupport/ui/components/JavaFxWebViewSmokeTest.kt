package com.malinatrash.camundasupport.ui.components

import java.awt.GraphicsEnvironment
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import javafx.application.Platform
import javafx.concurrent.Worker
import javafx.embed.swing.JFXPanel
import javafx.scene.Scene
import javafx.scene.web.WebView
import javax.swing.SwingUtilities
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.runBlocking
import netscape.javascript.JSObject

class JavaFxWebViewSmokeTest {
    @Test
    fun sharpBpmnLayerIsRenderedOnlyAfterTwoSecondsWithoutInteraction() {
        assertEquals(2_000, BPMN_SHARP_RENDER_DEBOUNCE_MILLIS)
        val html = BpmnHtml.build(
            svg = "<svg viewBox=\"0 0 100 100\"></svg>",
            activeIds = emptySet(),
            incidentIds = emptySet(),
            completedCounts = emptyMap(),
            clickableIds = emptySet(),
        )

        assertTrue(
            "setTimeout(renderSharpViewport, $BPMN_SHARP_RENDER_DEBOUNCE_MILLIS);" in html,
            "Чёткий BPMN-слой должен перерисовываться через две секунды без новых событий",
        )
    }

    @Test
    fun webViewStartsWithPackagedDependencies() {
        if (GraphicsEnvironment.isHeadless()) return

        SwingUtilities.invokeAndWait { JFXPanel() }
        Platform.setImplicitExit(false)
        val completed = CountDownLatch(1)
        val failure = AtomicReference<Throwable?>()

        Platform.runLater {
            runCatching {
                WebView().engine.apply {
                    isJavaScriptEnabled = true
                    loadContent("<html lang=\"ru\"><body>Проверка</body></html>", "text/html")
                }
            }.onFailure(failure::set)
            completed.countDown()
        }

        assertTrue(completed.await(10, TimeUnit.SECONDS), "JavaFX WebView не запустился за 10 секунд")
        assertNull(failure.get(), "JavaFX WebView не смог загрузить HTML")
    }

    @Test
    fun bpmnCubeClickIsDeliveredToKotlin() = runBlocking {
        if (GraphicsEnvironment.isHeadless()) return@runBlocking

        SwingUtilities.invokeAndWait { JFXPanel() }
        val completed = CountDownLatch(1)
        val failure = AtomicReference<Throwable?>()
        val clickedActivityId = AtomicReference<String?>()
        val webViewReference = AtomicReference<WebView?>()
        val svg = BpmnSvgRenderer.render(TEST_BPMN).diagram(null).svg
        val html = BpmnHtml.build(
            svg = svg,
            activeIds = setOf("Task_1"),
            incidentIds = emptySet(),
            completedCounts = mapOf("Task_1" to 2),
            clickableIds = setOf("Task_1"),
        )

        Platform.runLater {
            runCatching {
                val webView = WebView()
                webViewReference.set(webView)
                webView.engine.apply {
                    isJavaScriptEnabled = true
                    loadWorker.stateProperty().addListener { _, _, state ->
                        if (state == Worker.State.SUCCEEDED) {
                            runCatching {
                                val bridge = BpmnWebBridge(onClick = { activityId ->
                                    clickedActivityId.set(activityId)
                                    completed.countDown()
                                })
                                (executeScript("window") as JSObject).setMember("supportBridge", bridge)
                                executeScript(
                                    """
                                        window.supportTestTimer = setInterval(function() {
                                          if (window.supportDiagramError) {
                                            clearInterval(window.supportTestTimer);
                                            window.supportBridge.onActivityClick('__ERROR__:' + window.supportDiagramError);
                                          }
                                          if (window.supportDiagramReady && window.supportSelectActivity) {
                                            if (document.getElementById('source-host')) {
                                              clearInterval(window.supportTestTimer);
                                              window.supportBridge.onActivityClick('__ERROR__:Исходный SVG остался в интерактивном DOM');
                                              return;
                                            }
                                            if (!document.getElementById('diagram-bitmap')) {
                                              clearInterval(window.supportTestTimer);
                                              window.supportBridge.onActivityClick('__ERROR__:Bitmap-слой BPMN не создан');
                                              return;
                                            }
                                            if (document.querySelectorAll('.support-hit').length !== 1) {
                                              clearInterval(window.supportTestTimer);
                                              window.supportBridge.onActivityClick('__ERROR__:Карта кликов BPMN не создана');
                                              return;
                                            }
                                            if (!window.supportTestInteracted) {
                                              window.supportTestInteracted = true;
                                              for (var i = 0; i < 50; i++) {
                                                window.supportQueuePan(1, -1);
                                                window.supportQueueZoom(1.001);
                                              }
                                              return;
                                            }
                                            if (window.supportSharpReady) {
                                              clearInterval(window.supportTestTimer);
                                              if (!document.querySelector('#sharp-viewport img')) {
                                                window.supportBridge.onActivityClick('__ERROR__:Чёткий SVG-слой не создан');
                                                return;
                                              }
                                              window.supportSelectActivity('Task_1');
                                            }
                                          }
                                        }, 20);
                                    """.trimIndent(),
                                )
                            }.onFailure {
                                failure.set(it)
                                completed.countDown()
                            }
                        }
                    }
                    loadContent(html, "text/html")
                }
            }.onFailure {
                failure.set(it)
                completed.countDown()
            }
        }

        assertTrue(completed.await(30, TimeUnit.SECONDS), "Клик BPMN не пришёл из WebView за 30 секунд")
        assertNull(failure.get(), "WebView-мост BPMN завершился с ошибкой")
        assertEquals("Task_1", clickedActivityId.get())
        Platform.runLater { webViewReference.getAndSet(null)?.engine?.load(null) }
    }

    @Test
    fun nestedSubprocessDiagramsAreExportedSeparatelyAndRemainNavigable() = runBlocking {
        if (GraphicsEnvironment.isHeadless()) return@runBlocking

        val navigationPanel = AtomicReference<JFXPanel?>()
        SwingUtilities.invokeAndWait { navigationPanel.set(JFXPanel()) }
        val xml = requireNotNull(javaClass.getResourceAsStream("/drzi.bpmn"))
            .bufferedReader()
            .use { it.readText() }
        val rendered = BpmnSvgRenderer.renderAll(xml)

        assertEquals("drzi", rendered.rootElementId)
        assertEquals(
            setOf("drzi", "Activity_1gooqz2", "Activity_1xqooxf"),
            rendered.diagrams.keys,
        )
        assertEquals("drzi", rendered.diagrams.getValue("Activity_1gooqz2").parentElementId)
        assertEquals("drzi", rendered.diagrams.getValue("Activity_1xqooxf").parentElementId)
        assertTrue("Activity_1gooqz2" in rendered.diagram("drzi").elementIds)
        assertFalse("Activity_0soegtg" in rendered.diagram("drzi").elementIds)
        assertTrue("Activity_0soegtg" in rendered.diagram("Activity_1gooqz2").elementIds)
        assertTrue(rendered.diagram("Activity_1gooqz2").svg.isNotBlank())

        val html = BpmnHtml.build(
            svg = rendered.diagram("drzi").svg,
            activeIds = setOf("Activity_1gooqz2"),
            incidentIds = emptySet(),
            completedCounts = emptyMap(),
            clickableIds = setOf("Activity_1gooqz2"),
            navigationTargets = mapOf("Activity_1gooqz2" to "Подготовить изменения"),
            currentDiagramName = "drzi",
        )
        assertTrue("support-drilldown" in html)
        assertTrue("onDiagramNavigate" in html)
        assertTrue("Открыть подпроцесс" in html)

        val navigationCompleted = CountDownLatch(1)
        val navigationFailure = AtomicReference<Throwable?>()
        val navigatedDiagramId = AtomicReference<String?>()
        val navigationWebView = AtomicReference<WebView?>()
        Platform.runLater {
            runCatching {
                val webView = WebView()
                navigationWebView.set(webView)
                navigationPanel.get()?.scene = Scene(webView, 940.0, 260.0)
                webView.engine.apply {
                    isJavaScriptEnabled = true
                    loadWorker.stateProperty().addListener { _, _, state ->
                        if (state == Worker.State.SUCCEEDED) {
                            runCatching {
                                val bridge = BpmnWebBridge(
                                    onClick = {},
                                    onNavigate = { diagramId ->
                                        navigatedDiagramId.set(diagramId)
                                        navigationCompleted.countDown()
                                    },
                                )
                                (executeScript("window") as JSObject).setMember("supportBridge", bridge)
                                executeScript(
                                    """
                                        window.supportNavigationTest = setInterval(function() {
                                          if (!window.supportDiagramReady) return;
                                          clearInterval(window.supportNavigationTest);
                                          const button = document.querySelector('.support-drilldown');
                                          if (!button) {
                                            window.supportBridge.onDiagramNavigate('__ERROR__:Стрелка не создана');
                                            return;
                                          }
                                          const rect = button.getBoundingClientRect();
                                          const centerX = rect.left + rect.width / 2;
                                          const centerY = rect.top + rect.height / 2;
                                          const target = document.elementFromPoint(centerX, centerY);
                                          if (target !== button) {
                                            window.supportBridge.onDiagramNavigate(
                                              '__ERROR__:Стрелку перекрывает ' + (target ? target.tagName : 'пустой слой')
                                            );
                                            return;
                                          }
                                          ['mousedown', 'mouseup', 'click'].forEach(type => {
                                            target.dispatchEvent(new MouseEvent(type, {
                                              bubbles: true,
                                              clientX: centerX,
                                              clientY: centerY,
                                              button: 0
                                            }));
                                          });
                                        }, 20);
                                    """.trimIndent(),
                                )
                            }.onFailure {
                                navigationFailure.set(it)
                                navigationCompleted.countDown()
                            }
                        }
                    }
                    loadContent(html, "text/html")
                }
            }.onFailure {
                navigationFailure.set(it)
                navigationCompleted.countDown()
            }
        }
        assertTrue(navigationCompleted.await(15, TimeUnit.SECONDS), "Клик по стрелке не пришёл из WebView")
        assertNull(navigationFailure.get(), "Навигация BPMN завершилась с ошибкой")
        assertEquals("Activity_1gooqz2", navigatedDiagramId.get())
        Platform.runLater { navigationWebView.getAndSet(null)?.engine?.load(null) }
    }

    private companion object {
        val TEST_BPMN = """
            <?xml version="1.0" encoding="UTF-8"?>
            <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
              xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
              xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
              xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
              targetNamespace="http://example.com/test">
              <process id="Process_1" isExecutable="true">
                <startEvent id="Start_1" />
                <serviceTask id="Task_1" name="Проверяемый кубик" />
                <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Task_1" />
              </process>
              <bpmndi:BPMNDiagram id="Diagram_1">
                <bpmndi:BPMNPlane id="Plane_1" bpmnElement="Process_1">
                  <bpmndi:BPMNShape id="Start_shape" bpmnElement="Start_1">
                    <dc:Bounds x="100" y="100" width="36" height="36" />
                  </bpmndi:BPMNShape>
                  <bpmndi:BPMNShape id="Task_shape" bpmnElement="Task_1">
                    <dc:Bounds x="220" y="78" width="100" height="80" />
                  </bpmndi:BPMNShape>
                  <bpmndi:BPMNEdge id="Flow_edge" bpmnElement="Flow_1">
                    <di:waypoint x="136" y="118" />
                    <di:waypoint x="220" y="118" />
                  </bpmndi:BPMNEdge>
                </bpmndi:BPMNPlane>
              </bpmndi:BPMNDiagram>
            </definitions>
        """.trimIndent()
    }
}
