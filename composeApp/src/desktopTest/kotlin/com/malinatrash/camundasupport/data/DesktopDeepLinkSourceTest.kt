package com.malinatrash.camundasupport.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class DesktopDeepLinkSourceTest {
    @Test
    fun `parses Cockpit process instance URL`() {
        val result = parseDesktopDeepLink(
            "https://camunda-cf-pledge-loan.paas-dev.berekebank.kz/camunda/app/cockpit/default/" +
                "#/process-instance/1cf88e5f-8508-11f1-9c3c-b2aa3636eb4e/runtime?viewbox=%7B%7D",
        )

        val event = assertIs<DeepLinkEvent.OpenProcessInstance>(result)
        assertEquals("https://camunda-cf-pledge-loan.paas-dev.berekebank.kz", event.link.origin)
        assertEquals("1cf88e5f-8508-11f1-9c3c-b2aa3636eb4e", event.link.processInstanceId)
    }

    @Test
    fun `parses application deep link`() {
        val result = parseDesktopDeepLink(
            "camunda-support://process-instance/instance-42" +
                "?origin=https%3A%2F%2Fcamunda.example.kz",
        )

        val event = assertIs<DeepLinkEvent.OpenProcessInstance>(result)
        assertEquals("https://camunda.example.kz", event.link.origin)
        assertEquals("instance-42", event.link.processInstanceId)
    }

    @Test
    fun `rejects application link without Camunda origin`() {
        val result = parseDesktopDeepLink("camunda-support://process-instance/instance-42")

        assertIs<DeepLinkEvent.Invalid>(result)
    }
}
