package com.malinatrash.camundasupport.data

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.time.Instant
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.CurrentActivityEvidence
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.DashboardDatePreset
import com.malinatrash.camundasupport.model.DeploymentSort
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.model.TeleportRequest
import com.malinatrash.camundasupport.model.ProcessVariableUpdate
import com.malinatrash.camundasupport.model.VariableValueType

class DesktopCamundaApiTest {
    private lateinit var server: HttpServer

    @BeforeTest
    fun setUp() {
        server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
    }

    @AfterTest
    fun tearDown() {
        server.stop(0)
    }

    @Test
    fun dashboardJoinsLatestDefinitionsWithRuntimeStatistics() = runBlocking {
        val definitionQuery = AtomicReference<String>()
        server.createContext("/engine-rest/process-definition") { exchange ->
            definitionQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[
                    {"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null},
                    {"id":"beta:4:def","key":"beta","name":null,"version":4,"deploymentId":"dep-4","suspended":true,"tenantId":"b2c"}
                ]""".trimIndent(),
            )
        }
        server.createContext("/engine-rest/process-definition/statistics") { exchange ->
            exchange.respond(
                200,
                """[
                    {"id":"alpha:2:abc","instances":6,"incidents":[{"incidentType":"failedJob","incidentCount":2}]},
                    {"id":"beta:4:def","instances":24,"incidents":[{"incidentType":"failedJob","incidentCount":3},{"incidentType":"externalTask","incidentCount":1}]}
                ]""".trimIndent(),
            )
        }
        server.start()

        val result = DesktopCamundaApi().loadDashboard(
            connection(),
            DeploymentSort.NewestFirst,
            DashboardDateFilter(),
        )

        assertIs<CamundaApiResult.Success<*>>(result)
        val dashboard = result.value as com.malinatrash.camundasupport.model.ProcessDashboard
        assertEquals(2, dashboard.definitions.size)
        assertEquals(30, dashboard.instances)
        assertEquals(6, dashboard.incidents)
        assertEquals("beta", dashboard.definitions[1].displayName)
        assertTrue(dashboard.definitions[1].suspended)
        assertContains(definitionQuery.get(), "latestVersion=true")
        assertContains(definitionQuery.get(), "sortBy=deployTime")
        assertContains(definitionQuery.get(), "sortOrder=desc")
    }

    @Test
    fun variableSearchSendsTypedPostBodyAndParsesInstances() = runBlocking {
        val requestBody = AtomicReference<String>()
        server.createContext("/engine-rest/process-instance") { exchange ->
            assertEquals("POST", exchange.requestMethod)
            requestBody.set(exchange.requestBody.bufferedReader().readText())
            exchange.respond(
                200,
                """[{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","businessKey":"BR-42","ended":false,"suspended":false,"tenantId":"b2c"}]""",
            )
        }
        server.start()

        val result = DesktopCamundaApi().searchProcessInstances(
            connection = connection(),
            processDefinitionKey = "loan",
            variableName = "applicationId",
            variableValue = "42",
            valueType = VariableValueType.Number,
        )

        assertIs<CamundaApiResult.Success<*>>(result)
        val instances = result.value as List<*>
        assertEquals(1, instances.size)
        val json = Json.parseToJsonElement(requestBody.get()).jsonObject
        assertEquals("loan", json["processDefinitionKey"]!!.jsonPrimitive.content)
        val variable = json["variables"]!!.jsonArray.single().jsonObject
        assertEquals("applicationId", variable["name"]!!.jsonPrimitive.content)
        assertEquals("eq", variable["operator"]!!.jsonPrimitive.content)
        assertEquals("42", variable["value"]!!.jsonPrimitive.content)
        assertTrue(!variable["value"]!!.jsonPrimitive.isString)
    }

    @Test
    fun automaticVariableSearchTriesStringAndDetectedPrimitiveType() = runBlocking {
        val requests = AtomicInteger()
        server.createContext("/engine-rest/process-instance") { exchange ->
            requests.incrementAndGet()
            val body = Json.parseToJsonElement(exchange.requestBody.bufferedReader().readText()).jsonObject
            val value = body["variables"]!!.jsonArray.single().jsonObject["value"]!!.jsonPrimitive
            exchange.respond(
                200,
                if (value.isString) {
                    """[{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false}]"""
                } else {
                    "[]"
                },
            )
        }
        server.start()

        val result = DesktopCamundaApi().searchProcessInstances(
            connection = connection(),
            processDefinitionKey = "loan",
            variableName = "applicationId",
            variableValue = "42",
            valueType = VariableValueType.Auto,
        )

        val instances = assertIs<CamundaApiResult.Success<List<com.malinatrash.camundasupport.model.ProcessInstanceSummary>>>(result).value
        assertEquals(2, requests.get())
        assertEquals(listOf("instance-1"), instances.map { it.id })
    }

    @Test
    fun searchCatalogLoadsProcessesAndConcreteVariablesFromTheirActiveInstances() = runBlocking {
        val definitionQuery = AtomicReference<String>()
        val instanceQuery = AtomicReference<String>()
        val variableQuery = AtomicReference<String>()
        val variableRequests = AtomicInteger()
        server.createContext("/engine-rest/process-definition") { exchange ->
            definitionQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[
                    {"id":"loan:3:def","key":"loan","name":"Кредит","version":3,"deploymentId":"dep","suspended":false},
                    {"id":"pledge:1:def","key":"pledge","name":"Залог","version":1,"deploymentId":"dep","suspended":false}
                ]""".trimIndent(),
            )
        }
        server.createContext("/engine-rest/process-instance") { exchange ->
            assertEquals("GET", exchange.requestMethod)
            instanceQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[
                    {"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false},
                    {"id":"instance-2","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false}
                ]""".trimIndent(),
            )
        }
        server.createContext("/engine-rest/variable-instance") { exchange ->
            variableRequests.incrementAndGet()
            variableQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[
                    {"id":"v1","name":"applicationId","type":"String","processInstanceId":"instance-1"},
                    {"id":"v2","name":"applicationId","type":"String","processInstanceId":"instance-2"},
                    {"id":"v3","name":"amount","type":"Long","processInstanceId":"instance-2"}
                ]""".trimIndent(),
            )
        }
        server.start()

        val catalogRepository = InMemoryProcessVariableCatalogRepository()
        val api = DesktopCamundaApi(
            now = { Instant.parse("2026-07-24T00:00:00Z") },
            variableCatalogRepository = catalogRepository,
        )
        val definitions = assertIs<CamundaApiResult.Success<List<com.malinatrash.camundasupport.model.ProcessDefinitionSummary>>>(
            api.loadSearchProcessDefinitions(connection()),
        ).value
        val catalog = assertIs<CamundaApiResult.Success<com.malinatrash.camundasupport.model.ProcessVariableCatalog>>(
            api.loadProcessVariableCatalog(connection(), "loan"),
        ).value

        assertEquals(listOf("Залог", "Кредит"), definitions.map { it.displayName })
        assertContains(definitionQuery.get(), "latestVersion=true")
        assertContains(instanceQuery.get(), "processDefinitionKey=loan")
        assertContains(variableQuery.get(), "processInstanceIdIn=")
        assertContains(variableQuery.get(), "deserializeValues=false")
        assertEquals(2, catalog.inspectedInstanceCount)
        assertEquals(listOf("amount", "applicationId"), catalog.variables.map { it.name })
        assertEquals(2, catalog.variables.single { it.name == "applicationId" }.occurrences)

        api.loadProcessVariableCatalog(connection(), "loan")
        assertEquals(1, variableRequests.get(), "Повторное открытие процесса должно использовать кэш каталога")

        DesktopCamundaApi(
            now = { Instant.parse("2026-07-24T01:00:00Z") },
            variableCatalogRepository = catalogRepository,
        ).loadProcessVariableCatalog(connection(), "loan")
        assertEquals(1, variableRequests.get(), "Новый сеанс приложения должен использовать дисковый кэш каталога")
    }

    @Test
    fun forbiddenDashboardReturnsSpecificFailure() = runBlocking {
        server.createContext("/engine-rest/process-definition") { exchange -> exchange.respond(403, "Forbidden") }
        server.start()

        val result = DesktopCamundaApi().loadDashboard(
            connection(),
            DeploymentSort.NewestFirst,
            DashboardDateFilter(),
        )

        assertIs<CamundaApiResult.Failure>(result)
        assertContains(result.message, "Доступ запрещён (HTTP 403)")
    }

    @Test
    fun customPeriodUsesHistoricCountEndpoints() = runBlocking {
        server.createContext("/engine-rest/process-definition") { exchange ->
            exchange.respond(
                200,
                """[{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}]""",
            )
        }
        val processQueries = mutableListOf<String>()
        val incidentQuery = AtomicReference<String>()
        server.createContext("/engine-rest/history/process-instance/count") { exchange ->
            processQueries += exchange.requestURI.rawQuery
            exchange.respond(200, """{"count":17}""")
        }
        server.createContext("/engine-rest/history/incident/count") { exchange ->
            incidentQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(200, """{"count":3}""")
        }
        server.start()

        val result = DesktopCamundaApi().loadDashboard(
            connection = connection(),
            sort = DeploymentSort.NewestFirst,
            dateFilter = DashboardDateFilter(
                preset = DashboardDatePreset.Custom,
                fromDate = "2026-07-15",
                toDate = "2026-07-16",
            ),
        )

        val success = assertIs<CamundaApiResult.Success<*>>(result)
        val dashboard = success.value as com.malinatrash.camundasupport.model.ProcessDashboard
        assertEquals(17, dashboard.instances)
        assertEquals(3, dashboard.incidents)
        val definitionQuery = processQueries.first { "processDefinitionId=" in it }
        assertContains(definitionQuery, "processDefinitionId=alpha%3A2%3Aabc")
        assertContains(definitionQuery, "startedAfter=")
        assertContains(definitionQuery, "startedBefore=")
        assertContains(incidentQuery.get(), "createTimeAfter=")
        assertContains(incidentQuery.get(), "createTimeBefore=")
    }

    @Test
    fun dashboardBuildsStartedCompletedAndIncidentTimeline() = runBlocking {
        val activityQuery = AtomicReference<String>()
        server.createContext("/engine-rest/process-definition/alpha:2:abc/xml") { exchange ->
            exchange.respond(
                200,
                """{"bpmn20Xml":"<?xml version=\"1.0\" encoding=\"UTF-8\"?><definitions xmlns=\"http://www.omg.org/spec/BPMN/20100524/MODEL\" xmlns:bpmndi=\"http://www.omg.org/spec/BPMN/20100524/DI\" xmlns:dc=\"http://www.omg.org/spec/DD/20100524/DC\"><process id=\"alpha\"><serviceTask id=\"sendDocuments\" name=\"Отправить документы\" /></process><bpmndi:BPMNDiagram><bpmndi:BPMNPlane bpmnElement=\"alpha\"><bpmndi:BPMNShape bpmnElement=\"sendDocuments\"><dc:Bounds x=\"100\" y=\"100\" width=\"100\" height=\"80\" /></bpmndi:BPMNShape></bpmndi:BPMNPlane></bpmndi:BPMNDiagram></definitions>"}""",
            )
        }
        server.createContext("/engine-rest/process-definition") { exchange ->
            exchange.respond(
                200,
                """[{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}]""",
            )
        }
        server.createContext("/engine-rest/history/process-instance/count") { exchange ->
            val query = exchange.requestURI.rawQuery.orEmpty()
            exchange.respond(200, if ("completed=true" in query) """{"count":2}""" else """{"count":3}""")
        }
        server.createContext("/engine-rest/history/incident/count") { exchange ->
            exchange.respond(200, """{"count":2}""")
        }
        server.createContext("/engine-rest/history/process-instance") { exchange ->
            val completed = "completed=true" in exchange.requestURI.rawQuery.orEmpty()
            exchange.respond(
                200,
                if (completed) {
                    """[
                        {"id":"done-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T08:00:00.000+0800","endTime":"2026-07-15T08:30:00.000+0800","state":"COMPLETED"},
                        {"id":"done-2","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-16T08:00:00.000+0800","endTime":"2026-07-16T10:00:00.000+0800","state":"COMPLETED"}
                    ]""".trimIndent()
                } else {
                    """[
                        {"id":"started-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T08:00:00.000+0800"},
                        {"id":"started-2","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T11:00:00.000+0800"},
                        {"id":"started-3","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-16T08:00:00.000+0800"}
                    ]""".trimIndent()
                },
            )
        }
        server.createContext("/engine-rest/history/incident") { exchange ->
            exchange.respond(
                200,
                """[
                    {"id":"incident-1","incidentType":"failedJob","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:abc","processInstanceId":"started-1","createTime":"2026-07-15T09:00:00.000+0800"},
                    {"id":"incident-2","incidentType":"failedExternalTask","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:abc","processInstanceId":"started-3","createTime":"2026-07-16T09:00:00.000+0800"}
                ]""".trimIndent(),
            )
        }
        server.createContext("/engine-rest/history/activity-instance") { exchange ->
            activityQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[
                    {"id":"activity-ancient","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"ancient-300-days","startTime":"2025-09-19T08:00:00.000+0800"},
                    {"id":"activity-old","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"started-1","startTime":"2026-07-15T08:00:00.000+0800"},
                    {"id":"activity-new","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"started-3","startTime":"2026-07-16T08:00:00.000+0800"}
                ]""".trimIndent(),
            )
        }
        server.start()

        val result = DesktopCamundaApi(now = { Instant.parse("2026-07-16T12:00:00Z") }).loadDashboard(
            connection = connection(),
            sort = DeploymentSort.NewestFirst,
            dateFilter = DashboardDateFilter(
                preset = DashboardDatePreset.Custom,
                fromDate = "2026-07-15",
                toDate = "2026-07-16",
            ),
        )

        val dashboard = assertIs<CamundaApiResult.Success<*>>(result).value as com.malinatrash.camundasupport.model.ProcessDashboard
        assertEquals(3, dashboard.instances)
        assertEquals(2, dashboard.completedInstances)
        assertEquals(2, dashboard.incidents)
        assertEquals(3, dashboard.timeline.sumOf { it.started })
        assertEquals(2, dashboard.timeline.sumOf { it.completed })
        assertEquals(2, dashboard.timeline.sumOf { it.incidents })
        assertEquals(4_500_000L, dashboard.averageDurationMillis)
        assertEquals(4_500_000L, dashboard.medianDurationMillis)
        assertEquals(6_930_000L, dashboard.p95DurationMillis)
        assertEquals(1, dashboard.durationBuckets.single { it.label == "15–60 мин" }.count)
        assertEquals(1, dashboard.durationBuckets.single { it.label == "1–4 ч" }.count)
        assertEquals(setOf("failedJob", "failedExternalTask"), dashboard.incidentTypes.map { it.type }.toSet())
        assertEquals("sendDocuments", dashboard.incidentActivities.single().activityId)
        assertEquals("Отправить документы", dashboard.incidentActivities.single().activityName)
        assertEquals(2, dashboard.incidentActivities.single().count)
        assertContains(activityQuery.get(), "unfinished=true")
        assertEquals(1, dashboard.longestWaitingActivities.size)
        assertEquals("Отправить документы", dashboard.longestWaitingActivities.single().activityName)
        assertEquals(2, dashboard.longestWaitingActivities.single().instanceCount)
        assertEquals("started-1", dashboard.longestWaitingActivities.single().oldestProcessInstanceId)
        assertEquals(129_600_000L, dashboard.longestWaitingActivities.single().maximumWaitingMillis)
        assertEquals(86_400_000L, dashboard.longestWaitingActivities.single().averageWaitingMillis)
    }

    @Test
    fun incidentsCanBeFilteredByDayAndAllVersionsOfProcess() = runBlocking {
        val definitionQuery = AtomicReference<String>()
        val incidentQuery = AtomicReference<String>()
        server.createContext("/engine-rest/process-definition") { exchange ->
            definitionQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[
                    {"id":"alpha:3:new","key":"alpha","name":"Alpha","version":3,"versionTag":"release-3","deploymentId":"dep-3","suspended":false},
                    {"id":"alpha:2:old","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false}
                ]""".trimIndent(),
            )
        }
        server.createContext("/engine-rest/history/incident") { exchange ->
            incidentQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[{"id":"incident-1","incidentType":"failedJob","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:old","createTime":"2026-07-15T09:00:00.000+0800"}]""",
            )
        }
        server.start()

        val versionsResult = DesktopCamundaApi().loadProcessDefinitionVersions(connection(), "alpha")
        val versions = assertIs<CamundaApiResult.Success<*>>(versionsResult).value as List<*>
        assertEquals(2, versions.size)
        assertContains(definitionQuery.get(), "key=alpha")
        assertContains(definitionQuery.get(), "sortBy=version")

        val incidentsResult = DesktopCamundaApi().loadIncidents(
            connection = connection(),
            processDefinitionId = null,
            dateFilter = DashboardDateFilter(
                preset = DashboardDatePreset.Custom,
                fromDate = "2026-07-15",
                toDate = "2026-07-15",
            ),
            processDefinitionKey = "alpha",
        )
        assertEquals(1, (assertIs<CamundaApiResult.Success<*>>(incidentsResult).value as List<*>).size)
        assertContains(incidentQuery.get(), "processDefinitionKeyIn=alpha")
        assertContains(incidentQuery.get(), "createTimeAfter=")
        assertContains(incidentQuery.get(), "createTimeBefore=")
    }

    @Test
    fun teleportUsesActivityInstanceIdAndStartTarget() = runBlocking {
        val method = AtomicReference<String>()
        val path = AtomicReference<String>()
        val requestBody = AtomicReference<String>()
        server.createContext("/") { exchange ->
            method.set(exchange.requestMethod)
            path.set(exchange.requestURI.path)
            requestBody.set(exchange.requestBody.bufferedReader().readText())
            exchange.respondNoContent()
        }
        server.start()

        val result = DesktopCamundaApi().teleportProcessInstance(
            connection = connection(),
            processInstanceId = "instance-1",
            request = TeleportRequest(
                sourceActivityInstanceId = "activity-instance-7",
                targetActivityId = "Activity_target",
                annotation = "INC-42",
            ),
        )

        assertIs<CamundaApiResult.Success<Unit>>(result)
        assertEquals("POST", method.get())
        assertEquals("/engine-rest/process-instance/instance-1/modification", path.get())
        val payload = Json.parseToJsonElement(requestBody.get()).jsonObject
        assertEquals("INC-42", payload["annotation"]!!.jsonPrimitive.content)
        val instructions = payload["instructions"]!!.jsonArray
        assertEquals("activity-instance-7", instructions[0].jsonObject["activityInstanceId"]!!.jsonPrimitive.content)
        assertEquals("Activity_target", instructions[1].jsonObject["activityId"]!!.jsonPrimitive.content)
    }

    @Test
    fun recoveryOperationsUseExpectedEndpointsAndBodies() = runBlocking {
        val requests = mutableListOf<Triple<String, String, String>>()
        server.createContext("/") { exchange ->
            requests += Triple(
                exchange.requestMethod,
                exchange.requestURI.path,
                exchange.requestBody.bufferedReader().readText(),
            )
            exchange.respondNoContent()
        }
        server.start()
        val api = DesktopCamundaApi()

        assertIs<CamundaApiResult.Success<Unit>>(api.setJobRetries(connection(), "job-1", 3))
        assertIs<CamundaApiResult.Success<Unit>>(api.setExternalTaskRetries(connection(), "task-1", 3))
        assertIs<CamundaApiResult.Success<Unit>>(api.unlockExternalTask(connection(), "task-1"))
        assertIs<CamundaApiResult.Success<Unit>>(api.setProcessInstanceSuspended(connection(), "instance-1", true))

        assertEquals("PUT", requests[0].first)
        assertEquals("/engine-rest/job/job-1/retries", requests[0].second)
        assertEquals("3", Json.parseToJsonElement(requests[0].third).jsonObject["retries"]!!.jsonPrimitive.content)
        assertEquals("/engine-rest/external-task/task-1/retries", requests[1].second)
        assertEquals("POST", requests[2].first)
        assertEquals("/engine-rest/external-task/task-1/unlock", requests[2].second)
        assertEquals("/engine-rest/process-instance/instance-1/suspended", requests[3].second)
        assertEquals("true", Json.parseToJsonElement(requests[3].third).jsonObject["suspended"]!!.jsonPrimitive.content)
    }

    @Test
    fun variableUpdatePreservesTypeAndValueInfo() = runBlocking {
        val method = AtomicReference<String>()
        val path = AtomicReference<String>()
        val requestBody = AtomicReference<String>()
        server.createContext("/") { exchange ->
            method.set(exchange.requestMethod)
            path.set(exchange.requestURI.path)
            requestBody.set(exchange.requestBody.bufferedReader().readText())
            exchange.respondNoContent()
        }
        server.start()

        val result = DesktopCamundaApi().updateProcessVariable(
            connection = connection(),
            processInstanceId = "instance-1",
            update = ProcessVariableUpdate(
                name = "approved by support",
                type = "Boolean",
                value = "true",
                valueInfo = "{}",
            ),
        )

        assertIs<CamundaApiResult.Success<Unit>>(result)
        assertEquals("PUT", method.get())
        assertEquals("/engine-rest/process-instance/instance-1/variables/approved+by+support", path.get())
        val payload = Json.parseToJsonElement(requestBody.get()).jsonObject
        assertEquals("Boolean", payload["type"]!!.jsonPrimitive.content)
        assertEquals("true", payload["value"]!!.jsonPrimitive.content)
        assertTrue(!payload["value"]!!.jsonPrimitive.isString)
        assertEquals("{}", payload["valueInfo"]!!.jsonObject.toString())
    }

    @Test
    fun invalidJsonVariableIsRejectedBeforeRequest() = runBlocking {
        server.createContext("/") { exchange -> exchange.respondNoContent() }
        server.start()

        val result = DesktopCamundaApi().updateProcessVariable(
            connection = connection(),
            processInstanceId = "instance-1",
            update = ProcessVariableUpdate(
                name = "payload",
                type = "Json",
                value = "{broken",
                valueInfo = "{}",
            ),
        )

        val failure = assertIs<CamundaApiResult.Failure>(result)
        assertContains(failure.message, "корректный JSON")
    }

    @Test
    fun instanceDetailsCombineRuntimeStateVariablesFailuresAndBpmn() = runBlocking {
        val bpmnXml = """
            <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
                xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                xmlns:dc="http://www.omg.org/spec/DD/20100524/DC">
              <process id="loan">
                <serviceTask id="Task_1" name="Проверка" />
                <serviceTask id="Task_2" name="Ожидание перехода" />
                <userTask id="Task_4" name="Ожидание клиента" />
              </process>
              <bpmndi:BPMNDiagram id="d"><bpmndi:BPMNPlane id="p" bpmnElement="loan">
                <bpmndi:BPMNShape id="s" bpmnElement="Task_1"><dc:Bounds x="10" y="20" width="100" height="80" /></bpmndi:BPMNShape>
                <bpmndi:BPMNShape id="s2" bpmnElement="Task_2"><dc:Bounds x="150" y="20" width="100" height="80" /></bpmndi:BPMNShape>
                <bpmndi:BPMNShape id="s4" bpmnElement="Task_4"><dc:Bounds x="290" y="20" width="100" height="80" /></bpmndi:BPMNShape>
              </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
            </definitions>
        """.trimIndent()
        server.createContext("/") { exchange ->
            val response = when (exchange.requestURI.path) {
                "/engine-rest/process-instance/instance-1" ->
                    """{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","businessKey":"APP-42","ended":false,"suspended":false,"tenantId":"b2c"}"""
                "/engine-rest/process-instance/instance-1/variables" ->
                    """{"applicationId":{"type":"String","value":"APP-42","valueInfo":{}}}"""
                "/engine-rest/process-instance/instance-1/activity-instances" ->
                    """{"id":"root","activityId":"loan","activityType":"processDefinition","childActivityInstances":[{"id":"activity-instance-1","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","executionIds":["exec-1"],"incidentIds":["incident-1"]}],"childTransitionInstances":[{"id":"transition-1","activityId":"Task_2","activityName":"Ожидание перехода","activityType":"serviceTask","executionId":"exec-2","incidentIds":[]}]}"""
                "/engine-rest/history/activity-instance" ->
                    """[
                        {"id":"historic-1","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:00:00.000+0000","endTime":"2026-07-16T09:00:01.000+0000","canceled":false},
                        {"id":"historic-2","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:01:00.000+0000","endTime":"2026-07-16T09:01:01.000+0000","canceled":false},
                        {"id":"historic-3","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:02:00.000+0000","endTime":null,"canceled":false},
                        {"id":"historic-4","activityId":"Task_4","activityName":"Ожидание клиента","activityType":"userTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:03:00.000+0000","endTime":null,"canceled":false}
                    ]""".trimIndent()
                "/engine-rest/incident" ->
                    """[{"id":"incident-1","processInstanceId":"instance-1","incidentType":"failedJob","failedActivityId":"Task_1","incidentMessage":"boom"}]"""
                "/engine-rest/job" ->
                    """[{"id":"job-1","retries":0,"failedActivityId":"Task_1","exceptionMessage":"boom","suspended":false}]"""
                "/engine-rest/external-task" ->
                    """[{"id":"external-1","activityId":"Task_3","topicName":"loan-check","retries":0,"errorMessage":"worker failed","workerId":"worker-7","lockExpirationTime":"2026-07-16T10:00:00.000+0000","suspended":false}]"""
                "/engine-rest/process-definition/loan:3:def/xml" ->
                    """{"id":"loan:3:def","bpmn20Xml":${JsonPrimitive(bpmnXml)}}"""
                else -> error("Неожиданный путь: ${exchange.requestURI}")
            }
            exchange.respond(200, response)
        }
        server.start()

        val success = assertIs<CamundaApiResult.Success<*>>(
            DesktopCamundaApi().loadProcessInstanceDetails(connection(), "instance-1"),
        )
        val details = success.value as com.malinatrash.camundasupport.model.ProcessInstanceDetails

        assertEquals("APP-42", details.instance.businessKey)
        assertEquals("APP-42", details.variables.single().value)
        assertEquals("activity-instance-1", details.activeActivities.single { it.activityId == "Task_1" }.id)
        assertTrue(
            CurrentActivityEvidence.Incident in details.activeActivities.single { it.activityId == "Task_1" }.evidence,
        )
        assertTrue(
            CurrentActivityEvidence.Transition in details.activeActivities.single { it.activityId == "Task_2" }.evidence,
        )
        assertTrue(
            CurrentActivityEvidence.ExternalTask in details.activeActivities.single { it.activityId == "Task_3" }.evidence,
        )
        assertTrue(
            CurrentActivityEvidence.UnfinishedHistory in details.activeActivities.single { it.activityId == "Task_4" }.evidence,
        )
        assertTrue(!details.activeActivities.single { it.activityId == "Task_4" }.cancellable)
        assertTrue(!details.activeActivities.single { it.activityId == "Task_3" }.cancellable)
        assertEquals(2, details.activityHistory.single { it.activityId == "Task_1" }.completedCount)
        assertEquals(1, details.activityHistory.single { it.activityId == "Task_1" }.activeCount)
        assertEquals("boom", details.incidents.single().message)
        assertEquals(0, details.jobs.single().retries)
        assertEquals("worker-7", details.externalTasks.single().workerId)
        assertEquals(setOf("Task_1", "Task_2", "Task_4"), details.diagram.nodes.map { it.id }.toSet())
        assertEquals("b2c", details.metadata.single { it.label == "ID тенанта" }.value)
    }

    @Test
    fun definitionPeriodLoadsHistoricInstancesFromSelectedDates() = runBlocking {
        val instanceQuery = AtomicReference<String>()
        val incidentQuery = AtomicReference<String>()
        server.createContext("/engine-rest/process-definition/alpha:2:abc") { exchange ->
            exchange.respond(
                200,
                """{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}""",
            )
        }
        server.createContext("/engine-rest/history/process-instance") { exchange ->
            instanceQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(
                200,
                """[{"id":"historic-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","businessKey":"APP-YESTERDAY","startTime":"2026-07-15T10:00:00.000+0000","endTime":"2026-07-15T10:02:00.000+0000","state":"COMPLETED","tenantId":null}]""",
            )
        }
        server.createContext("/engine-rest/history/incident") { exchange ->
            incidentQuery.set(exchange.requestURI.rawQuery)
            exchange.respond(200, "[]")
        }
        server.start()

        val success = assertIs<CamundaApiResult.Success<*>>(
            DesktopCamundaApi().loadProcessDefinitionDetails(
                connection = connection(),
                processDefinitionId = "alpha:2:abc",
                dateFilter = DashboardDateFilter(preset = DashboardDatePreset.Yesterday),
            ),
        )
        val details = success.value as com.malinatrash.camundasupport.model.ProcessDefinitionDetails

        assertEquals("APP-YESTERDAY", details.instances.single().instance.businessKey)
        assertTrue(details.instances.single().instance.ended)
        assertContains(instanceQuery.get(), "startedAfter=")
        assertContains(instanceQuery.get(), "startedBefore=")
        assertContains(incidentQuery.get(), "sortBy=createTime")
        assertEquals(DashboardDatePreset.Yesterday, details.dateFilter.preset)
    }

    private fun connection() = CamundaConnection(
        id = "test",
        name = "Test",
        restUrl = "http://127.0.0.1:${server.address.port}/engine-rest",
        cockpitUrl = "http://127.0.0.1/camunda/app/cockpit",
        environment = Environment.Test,
    )

    private fun HttpExchange.respond(status: Int, body: String) {
        val bytes = body.encodeToByteArray()
        responseHeaders.add("Content-Type", "application/json")
        sendResponseHeaders(status, bytes.size.toLong())
        responseBody.use { it.write(bytes) }
    }

    private fun HttpExchange.respondNoContent() {
        sendResponseHeaders(204, -1)
        close()
    }
}
