package com.malinatrash.camundasupport.state

import com.malinatrash.camundasupport.data.InMemoryConnectionRepository
import com.malinatrash.camundasupport.model.AppDestination
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.model.ProcessInstanceSummary
import kotlin.test.Test
import kotlin.test.assertEquals

class AppStoreOpenProcessTabsTest {
    @Test
    fun keepsOpenInstancesAcrossConnectionsAndReturnsToThem() {
        val first = connection("first")
        val second = connection("second")
        val store = AppStore(InMemoryConnectionRepository(listOf(first, second)))

        store.openProcessInstance("instance-1")
        store.updateOpenProcessTab(instance("instance-1", "APP-1", "loan"))
        val firstTab = store.openProcessTabs.single()

        store.selectConnection(second.id)
        store.openProcessInstance("instance-2")
        store.updateOpenProcessTab(instance("instance-2", "APP-2", "pledge"))

        assertEquals(2, store.openProcessTabs.size)
        assertEquals("APP-2", store.openProcessTabs.last().businessKey)

        store.activateProcessTab(firstTab.id)

        assertEquals(first.id, store.selectedConnectionId)
        assertEquals("instance-1", store.selectedProcessInstanceId)
        assertEquals(AppDestination.ProcessInstance, store.destination)
        assertEquals(firstTab.id, store.activeProcessTabId)
    }

    @Test
    fun closingActiveTabSelectsAnotherOpenInstance() {
        val store = AppStore(InMemoryConnectionRepository(listOf(connection("first"))))
        store.openProcessInstance("instance-1")
        val firstTab = store.openProcessTabs.single()
        store.openProcessInstance("instance-2")
        val secondTab = store.openProcessTabs.last()

        store.closeProcessTab(secondTab.id)

        assertEquals(firstTab.id, store.activeProcessTabId)
        assertEquals("instance-1", store.selectedProcessInstanceId)
    }

    private fun connection(id: String) = CamundaConnection(
        id = id,
        name = "Подключение $id",
        restUrl = "https://$id.example.test/engine-rest",
        cockpitUrl = "https://$id.example.test/camunda/app/cockpit",
        environment = Environment.Test,
    )

    private fun instance(id: String, businessKey: String, definitionKey: String) = ProcessInstanceSummary(
        id = id,
        definitionId = "$definitionKey:1:def",
        definitionKey = definitionKey,
        businessKey = businessKey,
        suspended = false,
        tenantId = null,
    )
}
