package com.malinatrash.camundasupport.state

import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.model.BpmnDiagram
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
import com.malinatrash.camundasupport.model.ProcessVariableCatalog
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
class ProcessInstancePollingStoreTest {
    @Test
    fun returnsCachedDetailsUntilTwentySecondIntervalExpires() = runBlocking {
        val timeSource = TestTimeSource()
        val api = FakeCamundaApi()
        val store = ProcessInstancePollingStore(api, timeSource)
        val query = ProcessInstancePollingQuery(CONNECTION, "instance-1")

        store.refresh(query)
        val firstSnapshot = store.entry(query).details
        timeSource += 19.seconds
        store.refresh(query)

        assertEquals(1, api.detailsCalls)
        assertSame(firstSnapshot, store.entry(query).details)

        timeSource += 1.seconds
        store.refresh(query)

        assertEquals(2, api.detailsCalls)
    }

    @Test
    fun failedPollingKeepsLastSuccessfulStateVisible() = runBlocking {
        val api = FakeCamundaApi()
        val store = ProcessInstancePollingStore(api)
        val query = ProcessInstancePollingQuery(CONNECTION, "instance-1")

        store.refresh(query)
        val successfulSnapshot = store.entry(query).details
        api.failure = "Camunda временно недоступна"
        store.refresh(query, force = true)

        assertSame(successfulSnapshot, store.entry(query).details)
        assertEquals("Camunda временно недоступна", store.entry(query).error)
    }

    private class FakeCamundaApi : CamundaApi {
        var detailsCalls = 0
        var failure: String? = null

        override suspend fun loadProcessInstanceDetails(
            connection: CamundaConnection,
            processInstanceId: String,
        ): CamundaApiResult<ProcessInstanceDetails> {
            detailsCalls += 1
            return failure?.let { CamundaApiResult.Failure(it) }
                ?: CamundaApiResult.Success(
                    ProcessInstanceDetails(
                        instance = ProcessInstanceSummary(
                            id = processInstanceId,
                            definitionId = "loan:1:def",
                            definitionKey = "loan",
                            businessKey = "APP-$detailsCalls",
                            suspended = false,
                            tenantId = null,
                        ),
                        variables = emptyList(),
                        activeActivities = emptyList(),
                        activityHistory = emptyList(),
                        incidents = emptyList(),
                        jobs = emptyList(),
                        externalTasks = emptyList(),
                        diagram = BpmnDiagram(emptyList(), emptyList()),
                        bpmnXml = "",
                        metadata = emptyList(),
                    ),
                )
        }

        override suspend fun loadDashboard(
            connection: CamundaConnection,
            sort: DeploymentSort,
            dateFilter: DashboardDateFilter,
        ) = unused<ProcessDashboard>()

        override suspend fun searchProcessInstances(
            connection: CamundaConnection,
            processDefinitionKey: String,
            variableName: String,
            variableValue: String,
            valueType: VariableValueType,
        ) = unused<List<ProcessInstanceSummary>>()

        override suspend fun loadSearchProcessDefinitions(connection: CamundaConnection) =
            unused<List<ProcessDefinitionSummary>>()

        override suspend fun loadProcessVariableCatalog(
            connection: CamundaConnection,
            processDefinitionKey: String,
        ) = unused<ProcessVariableCatalog>()

        override suspend fun loadProcessDefinitionDetails(
            connection: CamundaConnection,
            processDefinitionId: String,
            dateFilter: DashboardDateFilter,
        ) = unused<ProcessDefinitionDetails>()

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

        private fun <T> unused(): CamundaApiResult<T> = CamundaApiResult.Failure("Метод не используется")
    }

    private companion object {
        val CONNECTION = CamundaConnection(
            id = "test",
            name = "Тест",
            restUrl = "https://example.test/engine-rest",
            cockpitUrl = "https://example.test/camunda/app/cockpit",
            environment = Environment.Test,
            health = ConnectionHealth.Connected,
        )
    }
}
