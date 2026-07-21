package com.malinatrash.camundasupport.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class DeepLinksTest {
    @Test
    fun `creates app link from connection origin`() {
        val connection = connection(
            restUrl = "https://camunda.example.kz/engine-rest",
            cockpitUrl = "https://camunda.example.kz/camunda/app/cockpit",
        )

        assertEquals(
            "camunda-support://process-instance/instance-42?origin=https%3A%2F%2Fcamunda.example.kz",
            connection.processInstanceDeepLink("instance-42"),
        )
    }

    @Test
    fun `matches exact normalized origin only`() {
        val connection = connection(
            restUrl = "https://CAMUNDA.example.kz:443/engine-rest",
            cockpitUrl = "",
        )

        assertTrue(connection.matchesOrigin("https://camunda.example.kz"))
        assertFalse(connection.matchesOrigin("https://other.example.kz"))
        assertFalse(connection.matchesOrigin("http://camunda.example.kz"))
    }

    @Test
    fun `rejects unsafe process instance id`() {
        val connection = connection("https://camunda.example.kz/engine-rest", "")

        assertNull(connection.processInstanceDeepLink("instance/42"))
        assertNull(connection.processInstanceDeepLink("instance 42"))
    }

    private fun connection(restUrl: String, cockpitUrl: String) = CamundaConnection(
        id = "test",
        name = "Тест",
        restUrl = restUrl,
        cockpitUrl = cockpitUrl,
        environment = Environment.Test,
    )
}
