package com.malinatrash.camundasupport.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class CockpitUrlsTest {
    @Test
    fun buildsDefaultEngineProcessDefinitionUrl() {
        val connection = connection("http://localhost:18080/camunda/app/cockpit/")

        assertEquals(
            "http://localhost:18080/camunda/app/cockpit/default/#/process-definition/process:2:id",
            connection.processDefinitionCockpitUrl("process:2:id"),
        )
    }

    @Test
    fun buildsRuntimeProcessInstanceUrl() {
        val connection = connection("http://localhost:18080/camunda/app/cockpit")

        assertEquals(
            "http://localhost:18080/camunda/app/cockpit/default/#/process-instance/instance-id/runtime",
            connection.processInstanceCockpitUrl("instance-id"),
        )
    }

    @Test
    fun buildsCockpitDashboardUrl() {
        val connection = connection("http://localhost:18080/camunda/app/cockpit/")

        assertEquals(
            "http://localhost:18080/camunda/app/cockpit/default/#/dashboard",
            connection.cockpitDashboardUrl(),
        )
    }

    @Test
    fun blankCockpitUrlCannotBuildLink() {
        assertNull(connection("").processDefinitionCockpitUrl("definition-id"))
    }

    @Test
    fun derivesCockpitUrlFromRestDomain() {
        assertEquals(
            "https://camunda.example.kz/camunda/app/cockpit",
            deriveCockpitUrl("https://camunda.example.kz/engine-rest"),
        )
        assertEquals(
            "http://localhost:18080/camunda/app/cockpit",
            deriveCockpitUrl("http://localhost:18080/custom/path/engine-rest"),
        )
    }

    private fun connection(cockpitUrl: String) = CamundaConnection(
        id = "test",
        name = "Test",
        restUrl = "http://localhost:18080/engine-rest",
        cockpitUrl = cockpitUrl,
        environment = Environment.Test,
    )
}
