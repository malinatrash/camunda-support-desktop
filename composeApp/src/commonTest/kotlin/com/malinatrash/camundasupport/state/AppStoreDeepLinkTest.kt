package com.malinatrash.camundasupport.state

import com.malinatrash.camundasupport.data.InMemoryConnectionRepository
import com.malinatrash.camundasupport.model.AppDestination
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.model.ProcessInstanceDeepLink
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class AppStoreDeepLinkTest {
    @Test
    fun `selects connection by origin and opens process instance`() {
        val pledge = connection("pledge", "https://camunda-pledge.example.kz")
        val b2c = connection("b2c", "https://camunda-b2c.example.kz")
        val store = AppStore(InMemoryConnectionRepository(listOf(b2c, pledge)))

        val result = store.openDeepLink(
            ProcessInstanceDeepLink("https://camunda-pledge.example.kz", "instance-42"),
        )

        assertIs<DeepLinkNavigationResult.Opened>(result)
        assertEquals(pledge.id, store.selectedConnectionId)
        assertEquals("instance-42", store.selectedProcessInstanceId)
        assertEquals(AppDestination.ProcessInstance, store.destination)
    }

    @Test
    fun `does not guess connection for unknown origin`() {
        val store = AppStore(InMemoryConnectionRepository(listOf(connection("b2c", "https://camunda-b2c.example.kz"))))

        val result = store.openDeepLink(
            ProcessInstanceDeepLink("https://unknown.example.kz", "instance-42"),
        )

        assertIs<DeepLinkNavigationResult.ConnectionNotFound>(result)
        assertEquals(AppDestination.Overview, store.destination)
    }

    private fun connection(id: String, origin: String) = CamundaConnection(
        id = id,
        name = id,
        restUrl = "$origin/engine-rest",
        cockpitUrl = "$origin/camunda/app/cockpit",
        environment = Environment.Test,
    )
}
