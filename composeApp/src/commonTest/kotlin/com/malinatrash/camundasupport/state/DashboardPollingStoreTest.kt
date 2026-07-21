package com.malinatrash.camundasupport.state

import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.ConnectionHealth
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.DeploymentSort
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.model.ProcessDashboard
import com.malinatrash.camundasupport.model.ProcessDefinitionDetails
import com.malinatrash.camundasupport.model.ProcessDefinitionSummary
import com.malinatrash.camundasupport.model.ProcessIncident
import com.malinatrash.camundasupport.model.ProcessInstanceDetails
import com.malinatrash.camundasupport.model.ProcessInstanceSummary
import com.malinatrash.camundasupport.model.ProcessVariableUpdate
import com.malinatrash.camundasupport.model.TeleportRequest
import com.malinatrash.camundasupport.model.VariableValueType
import kotlinx.coroutines.runBlocking
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame
import kotlin.time.Duration.Companion.seconds
import kotlin.time.ExperimentalTime
import kotlin.time.TestTimeSource

@OptIn(ExperimentalTime::class)
class DashboardPollingStoreTest {
    @Test
    fun freshSnapshotIsReturnedWithoutAnotherRequest() = runBlocking {
        val timeSource = TestTimeSource()
        val api = FakeCamundaApi()
        val store = DashboardPollingStore(api, timeSource)
        store.activate(CONNECTION, DashboardDateFilter())

        store.refreshActive()
        val firstSnapshot = store.entry(CONNECTION, DashboardDateFilter()).dashboard
        store.refreshActive()
        timeSource += 9.seconds
        store.refreshActive()

        assertEquals(1, api.dashboardCalls)
        assertSame(firstSnapshot, store.entry(CONNECTION, DashboardDateFilter()).dashboard)

        timeSource += 1.seconds
        store.refreshActive()
        assertEquals(2, api.dashboardCalls)
    }

    @Test
    fun forcedRefreshUpdatesCacheImmediately() = runBlocking {
        val api = FakeCamundaApi()
        val store = DashboardPollingStore(api)
        store.activate(CONNECTION, DashboardDateFilter())

        store.refreshActive()
        store.refreshActive(force = true)

        assertEquals(2, api.dashboardCalls)
        assertEquals(
            2,
            store.entry(CONNECTION, DashboardDateFilter()).dashboard?.completedInstances,
        )
    }

    @Test
    fun failedPollingKeepsLastSuccessfulSnapshot() = runBlocking {
        val timeSource = TestTimeSource()
        val api = FakeCamundaApi()
        val store = DashboardPollingStore(api, timeSource)
        store.activate(CONNECTION, DashboardDateFilter())
        store.refreshActive()
        val successfulSnapshot = store.entry(CONNECTION, DashboardDateFilter()).dashboard

        api.failure = "Camunda временно недоступна"
        timeSource += 10.seconds
        store.refreshActive()
        val entry = store.entry(CONNECTION, DashboardDateFilter())

        assertSame(successfulSnapshot, entry.dashboard)
        assertEquals("Camunda временно недоступна", entry.error)
    }

    private class FakeCamundaApi : CamundaApi {
        var dashboardCalls = 0
        var failure: String? = null

        override suspend fun loadDashboard(
            connection: CamundaConnection,
            sort: DeploymentSort,
            dateFilter: DashboardDateFilter,
        ): CamundaApiResult<ProcessDashboard> {
            dashboardCalls += 1
            return failure?.let { CamundaApiResult.Failure(it) }
                ?: CamundaApiResult.Success(ProcessDashboard(emptyList(), completedInstances = dashboardCalls))
        }

        override suspend fun searchProcessInstances(
            connection: CamundaConnection,
            variableName: String,
            variableValue: String,
            valueType: VariableValueType,
        ) = unused<List<ProcessInstanceSummary>>()

        override suspend fun loadProcessDefinitionDetails(
            connection: CamundaConnection,
            processDefinitionId: String,
            dateFilter: DashboardDateFilter,
        ) = unused<ProcessDefinitionDetails>()

        override suspend fun loadProcessInstanceDetails(connection: CamundaConnection, processInstanceId: String) =
            unused<ProcessInstanceDetails>()

        override suspend fun teleportProcessInstance(
            connection: CamundaConnection,
            processInstanceId: String,
            request: TeleportRequest,
        ) = unused<Unit>()

        override suspend fun setJobRetries(connection: CamundaConnection, jobId: String, retries: Int) = unused<Unit>()

        override suspend fun setExternalTaskRetries(
            connection: CamundaConnection,
            externalTaskId: String,
            retries: Int,
        ) = unused<Unit>()

        override suspend fun unlockExternalTask(connection: CamundaConnection, externalTaskId: String) = unused<Unit>()

        override suspend fun setProcessInstanceSuspended(
            connection: CamundaConnection,
            processInstanceId: String,
            suspended: Boolean,
        ) = unused<Unit>()

        override suspend fun updateProcessVariable(
            connection: CamundaConnection,
            processInstanceId: String,
            update: ProcessVariableUpdate,
        ) = unused<Unit>()

        override suspend fun loadIncidents(
            connection: CamundaConnection,
            processDefinitionId: String?,
            dateFilter: DashboardDateFilter,
            processDefinitionKey: String?,
        ) = unused<List<ProcessIncident>>()

        override suspend fun loadProcessDefinitionVersions(
            connection: CamundaConnection,
            processDefinitionKey: String,
        ) = unused<List<ProcessDefinitionSummary>>()

        private fun <T> unused(): CamundaApiResult<T> = CamundaApiResult.Failure("Метод не используется в тесте")
    }

    private companion object {
        val CONNECTION = CamundaConnection(
            id = "test",
            name = "Тест",
            restUrl = "http://localhost:8080/engine-rest",
            cockpitUrl = "http://localhost:8080/camunda/app/cockpit",
            environment = Environment.Test,
            health = ConnectionHealth.Connected,
        )
    }
}
