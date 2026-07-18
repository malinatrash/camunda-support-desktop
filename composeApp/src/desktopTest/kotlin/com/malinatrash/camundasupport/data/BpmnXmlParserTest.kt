package com.malinatrash.camundasupport.data

import kotlin.test.Test
import kotlin.test.assertEquals

class BpmnXmlParserTest {
    @Test
    fun parsesDiagramCoordinatesAndTeleportTargets() {
        val diagram = BpmnXmlParser().parse(
            """
                <?xml version="1.0" encoding="UTF-8"?>
                <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                    xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                    xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
                    xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
                    xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
                  <process id="Process_1">
                    <startEvent id="Start_1" name="Старт" />
                    <serviceTask id="Task_1" name="Проверить клиента" camunda:type="external" camunda:topic="customer-check" />
                    <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Task_1" />
                  </process>
                  <bpmndi:BPMNDiagram id="Diagram_1">
                    <bpmndi:BPMNPlane id="Plane_1" bpmnElement="Process_1">
                      <bpmndi:BPMNShape id="Start_shape" bpmnElement="Start_1">
                        <dc:Bounds x="100" y="120" width="36" height="36" />
                      </bpmndi:BPMNShape>
                      <bpmndi:BPMNShape id="Task_shape" bpmnElement="Task_1">
                        <dc:Bounds x="200" y="90" width="110" height="80" />
                      </bpmndi:BPMNShape>
                      <bpmndi:BPMNEdge id="Flow_edge" bpmnElement="Flow_1">
                        <di:waypoint x="136" y="138" />
                        <di:waypoint x="200" y="130" />
                      </bpmndi:BPMNEdge>
                    </bpmndi:BPMNPlane>
                  </bpmndi:BPMNDiagram>
                </definitions>
            """.trimIndent(),
        )

        assertEquals(listOf("Start_1", "Task_1"), diagram.nodes.map { it.id })
        assertEquals("serviceTask", diagram.nodes.last().type)
        assertEquals("customer-check", diagram.nodes.last().topic)
        assertEquals(2, diagram.edges.single().waypoints.size)
        assertEquals(listOf("Task_1"), diagram.teleportTargets.map { it.id })
    }
}
