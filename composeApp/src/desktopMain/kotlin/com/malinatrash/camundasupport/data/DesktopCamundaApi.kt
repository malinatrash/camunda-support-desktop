package com.malinatrash.camundasupport.data

import java.io.IOException
import java.net.ConnectException
import java.net.URI
import java.net.URLEncoder
import java.net.UnknownHostException
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.net.http.HttpTimeoutException
import java.nio.charset.StandardCharsets
import java.time.Duration
import java.time.Instant
import java.time.LocalDate
import java.time.OffsetDateTime
import java.time.ZonedDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeParseException
import java.time.temporal.ChronoUnit
import java.util.concurrent.ConcurrentHashMap
import javax.net.ssl.SSLException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.ActiveActivityInstance
import com.malinatrash.camundasupport.model.ActivityExecutionSummary
import com.malinatrash.camundasupport.model.BpmnDiagram
import com.malinatrash.camundasupport.model.CurrentActivityEvidence
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.DashboardDatePreset
import com.malinatrash.camundasupport.model.DashboardIncidentType
import com.malinatrash.camundasupport.model.DashboardIncidentActivity
import com.malinatrash.camundasupport.model.DashboardDurationBucket
import com.malinatrash.camundasupport.model.DashboardTimelinePoint
import com.malinatrash.camundasupport.model.DashboardWaitingActivity
import com.malinatrash.camundasupport.model.DeploymentSort
import com.malinatrash.camundasupport.model.ProcessDashboard
import com.malinatrash.camundasupport.model.ProcessDefinitionSummary
import com.malinatrash.camundasupport.model.ProcessDefinitionDetails
import com.malinatrash.camundasupport.model.ProcessExternalTask
import com.malinatrash.camundasupport.model.ProcessIncident
import com.malinatrash.camundasupport.model.ProcessInstanceDetails
import com.malinatrash.camundasupport.model.ProcessInstanceSummary
import com.malinatrash.camundasupport.model.ProcessJob
import com.malinatrash.camundasupport.model.ProcessMetadataField
import com.malinatrash.camundasupport.model.ProcessVariable
import com.malinatrash.camundasupport.model.ProcessVariableCatalog
import com.malinatrash.camundasupport.model.ProcessVariableDescriptor
import com.malinatrash.camundasupport.model.ProcessVariableUpdate
import com.malinatrash.camundasupport.model.RuntimeInstanceListItem
import com.malinatrash.camundasupport.model.TeleportRequest
import com.malinatrash.camundasupport.model.VariableValueType

class DesktopCamundaApi(
    private val client: HttpClient = HttpClient.newBuilder()
        .connectTimeout(CONNECT_TIMEOUT)
        .followRedirects(HttpClient.Redirect.NORMAL)
        .build(),
    private val bpmnXmlParser: BpmnXmlParser = BpmnXmlParser(),
    private val now: () -> Instant = Instant::now,
    private val variableCatalogRepository: ProcessVariableCatalogRepository =
        InMemoryProcessVariableCatalogRepository(),
) : CamundaApi {
    private val bpmnCache = ConcurrentHashMap<String, LoadedBpmn>()
    private val variableCatalogCache = ConcurrentHashMap<String, CachedVariableCatalog>()

    override suspend fun loadDashboard(
        connection: CamundaConnection,
        sort: DeploymentSort,
        dateFilter: DashboardDateFilter,
    ): CamundaApiResult<ProcessDashboard> = withContext(Dispatchers.IO) {
        val sortOrder = if (sort == DeploymentSort.NewestFirst) "desc" else "asc"
        val definitionsUrl = "${connection.restUrl}/process-definition" +
            "?latestVersion=true&sortBy=deployTime&sortOrder=$sortOrder"
        val statisticsUrl = "${connection.restUrl}/process-definition/statistics?incidents=true"

        apiCall("load process dashboard", definitionsUrl) {
            coroutineScope {
                val historyRange = dateFilter.toDashboardRange()
                val completedCountUrl = "${connection.restUrl}/history/process-instance/count" +
                    "?completed=true&finishedAfter=${encode(historyRange.after)}&finishedBefore=${encode(historyRange.before)}"
                val emptyPage = { PagedResult<ProcessInstanceSummary>(emptyList(), truncated = false) }
                val emptyIncidentPage = { PagedResult<ProcessIncident>(emptyList(), truncated = false) }

                val definitionsDeferred = async { parseDefinitions(send(get(definitionsUrl))) }
                val startedDeferred = async {
                    runCatching { loadHistoricInstances(connection, historyRange, completed = false) }
                        .getOrElse { emptyPage() }
                }
                val completedDeferred = async {
                    runCatching { loadHistoricInstances(connection, historyRange, completed = true) }
                        .getOrElse { emptyPage() }
                }
                val historicIncidentsDeferred = async {
                    runCatching { loadHistoricIncidents(connection, historyRange) }
                        .getOrElse { emptyIncidentPage() }
                }
                val statisticsDeferred = async {
                    if (dateFilter.isLive) {
                        parseStatistics(send(get(statisticsUrl)))
                    } else {
                        loadHistoricStatistics(connection, definitionsDeferred.await(), dateFilter)
                    }
                }
                val runtimeIncidentsDeferred: Deferred<List<ProcessIncident>>? = if (dateFilter.isLive) {
                    async {
                        runCatching { loadRuntimeIncidents(connection, processDefinitionId = null) }
                            .getOrDefault(emptyList())
                    }
                } else {
                    null
                }
                val unfinishedActivitiesDeferred = async {
                    runCatching { loadUnfinishedActivities(connection) }
                        .getOrElse { PagedResult(emptyList(), truncated = false) }
                }
                val completedCountDeferred = async {
                    runCatching { parseCount(send(get(completedCountUrl))) }.getOrNull()
                }

                val definitions = definitionsDeferred.await()
                val started = startedDeferred.await()
                val completed = completedDeferred.await()
                val historicIncidents = historicIncidentsDeferred.await()
                val statistics = statisticsDeferred.await()
                val unfinishedActivities = unfinishedActivitiesDeferred.await()
                val incidentsForBreakdown = runtimeIncidentsDeferred?.await() ?: historicIncidents.values
                val processInstanceIdsFromSelectedPeriod = started.values
                    .mapTo(mutableSetOf(), ProcessInstanceSummary::id)
                val incidentActivitiesDeferred = async {
                    loadIncidentActivities(connection, incidentsForBreakdown)
                }
                val longestWaitingActivitiesDeferred = async {
                    loadLongestWaitingActivities(
                        connection = connection,
                        activities = unfinishedActivities.values.filter { activity ->
                            activity.processInstanceId in processInstanceIdsFromSelectedPeriod
                        },
                    )
                }
                val completedDurations = completed.values.mapNotNull(::durationMillis).sorted()

                ProcessDashboard(
                    definitions = definitions.map { definition ->
                        val counts = statistics[definition.id] ?: DefinitionStatistics()
                        definition.copy(
                            instances = counts.instances,
                            incidents = counts.incidents,
                        )
                    },
                    timeline = buildTimeline(
                        range = historyRange,
                        started = started.values,
                        completed = completed.values,
                        incidents = historicIncidents.values,
                    ),
                    incidentTypes = incidentsForBreakdown
                        .groupingBy(ProcessIncident::type)
                        .eachCount()
                        .map { (type, count) -> DashboardIncidentType(type, count) }
                        .sortedByDescending(DashboardIncidentType::count),
                    incidentActivities = incidentActivitiesDeferred.await(),
                    longestWaitingActivities = longestWaitingActivitiesDeferred.await(),
                    durationBuckets = buildDurationBuckets(completedDurations),
                    timelineTitle = dateFilter.timelineTitle(),
                    averageDurationMillis = completedDurations
                        .takeIf { it.isNotEmpty() }
                        ?.average()
                        ?.toLong(),
                    medianDurationMillis = completedDurations.percentile(0.50),
                    p95DurationMillis = completedDurations.percentile(0.95),
                    completedInstances = completedCountDeferred.await() ?: completed.values.size,
                    timelineTruncated = started.truncated || completed.truncated ||
                        historicIncidents.truncated || unfinishedActivities.truncated,
                )
            }
        }
    }

    override suspend fun loadIncidents(
        connection: CamundaConnection,
        processDefinitionId: String?,
        dateFilter: DashboardDateFilter,
        processDefinitionKey: String?,
    ): CamundaApiResult<List<ProcessIncident>> = withContext(Dispatchers.IO) {
        val diagnosticUrl = "${connection.restUrl}/incident"
        apiCall("загрузка инцидентов", diagnosticUrl) {
            val incidents = if (dateFilter.isLive) {
                loadRuntimeIncidents(connection, processDefinitionId, processDefinitionKey)
            } else {
                loadHistoricIncidents(
                    connection = connection,
                    range = dateFilter.toCamundaDateRange(),
                    processDefinitionId = processDefinitionId,
                    processDefinitionKey = processDefinitionKey,
                ).values
            }
            incidents.sortedByDescending { parseCamundaDateTime(it.timestamp)?.toInstant() }
        }
    }

    override suspend fun loadProcessDefinitionVersions(
        connection: CamundaConnection,
        processDefinitionKey: String,
    ): CamundaApiResult<List<ProcessDefinitionSummary>> = withContext(Dispatchers.IO) {
        val url = "${connection.restUrl}/process-definition" +
            "?key=${encode(processDefinitionKey)}&sortBy=version&sortOrder=desc"
        apiCall("загрузка версий процесса", url) {
            parseDefinitions(send(get(url)))
        }
    }

    override suspend fun searchProcessInstances(
        connection: CamundaConnection,
        processDefinitionKey: String,
        variableName: String,
        variableValue: String,
        valueType: VariableValueType,
    ): CamundaApiResult<List<ProcessInstanceSummary>> = withContext(Dispatchers.IO) {
        val url = "${connection.restUrl}/process-instance?firstResult=0&maxResults=$SEARCH_LIMIT"
        apiCall("search process instances", url) {
            coroutineScope {
                variableValue.toSearchJsonValues(valueType).map { typedValue ->
                    async {
                        val body = buildJsonObject {
                            put("processDefinitionKey", processDefinitionKey)
                            put("variables", buildJsonArray {
                                add(buildJsonObject {
                                    put("name", variableName.trim())
                                    put("operator", "eq")
                                    put("value", typedValue)
                                })
                            })
                            put("sorting", buildJsonArray {
                                add(buildJsonObject {
                                    put("sortBy", "instanceId")
                                    put("sortOrder", "desc")
                                })
                            })
                        }
                        parseInstances(send(post(url, body.toString())))
                    }
                }.awaitAll()
                    .flatten()
                    .distinctBy(ProcessInstanceSummary::id)
                    .take(SEARCH_LIMIT)
            }
        }
    }

    override suspend fun loadSearchProcessDefinitions(
        connection: CamundaConnection,
    ): CamundaApiResult<List<ProcessDefinitionSummary>> = withContext(Dispatchers.IO) {
        val url = "${connection.restUrl}/process-definition" +
            "?latestVersion=true&sortBy=key&sortOrder=asc&maxResults=$SEARCH_DEFINITION_LIMIT"
        apiCall("загрузка процессов для поиска", url) {
            parseDefinitions(send(get(url)))
                .distinctBy { it.key to it.tenantId }
                .sortedBy { it.displayName.lowercase() }
        }
    }

    override suspend fun loadProcessVariableCatalog(
        connection: CamundaConnection,
        processDefinitionKey: String,
    ): CamundaApiResult<ProcessVariableCatalog> = withContext(Dispatchers.IO) {
        val normalizedKey = processDefinitionKey.trim()
        val cacheKey = "${connection.restUrl}\n$normalizedKey"
        val cached = variableCatalogCache[cacheKey]
        if (cached != null && Duration.between(cached.loadedAt, now()) < VARIABLE_CATALOG_CACHE_TTL) {
            return@withContext CamundaApiResult.Success(cached.catalog)
        }
        val persisted = variableCatalogRepository.load(connection.restUrl, normalizedKey)
        if (persisted != null) {
            val persistedAt = Instant.ofEpochMilli(persisted.loadedAtEpochMillis)
            if (Duration.between(persistedAt, now()) < VARIABLE_CATALOG_CACHE_TTL) {
                variableCatalogCache[cacheKey] = CachedVariableCatalog(persisted.catalog, persistedAt)
                return@withContext CamundaApiResult.Success(persisted.catalog)
            }
        }

        val instancesUrl = "${connection.restUrl}/process-instance" +
            "?processDefinitionKey=${encode(normalizedKey)}&firstResult=0&maxResults=${VARIABLE_CATALOG_INSTANCE_LIMIT + 1}"
        apiCall("загрузка переменных процесса", instancesUrl) {
            val loadedInstances = parseInstances(send(get(instancesUrl)))
            val instancesTruncated = loadedInstances.size > VARIABLE_CATALOG_INSTANCE_LIMIT
            val instances = loadedInstances.take(VARIABLE_CATALOG_INSTANCE_LIMIT)
            val variables = coroutineScope {
                val semaphore = Semaphore(MAX_PARALLEL_HISTORY_REQUESTS)
                instances.chunked(VARIABLE_INSTANCE_ID_BATCH_SIZE).map { batch ->
                    async {
                        semaphore.withPermit {
                            val ids = encode(batch.joinToString(",") { it.id })
                            loadPaged(
                                baseUrl = "${connection.restUrl}/variable-instance" +
                                    "?processInstanceIdIn=$ids&deserializeValues=false&sortBy=variableName&sortOrder=asc",
                                parser = ::parseVariableDescriptors,
                            ).values
                        }
                    }
                }.awaitAll().flatten()
            }
            val descriptors = variables
                .groupBy(VariableDescriptorSample::name)
                .map { (name, samples) ->
                    ProcessVariableDescriptor(
                        name = name,
                        types = samples.map(VariableDescriptorSample::type).distinct().sorted(),
                        occurrences = samples.size,
                    )
                }
                .sortedBy { it.name.lowercase() }
            val catalog = ProcessVariableCatalog(
                processDefinitionKey = normalizedKey,
                variables = descriptors,
                inspectedInstanceCount = instances.size,
                instancesTruncated = instancesTruncated,
            )
            if (variableCatalogCache.size >= VARIABLE_CATALOG_CACHE_LIMIT) variableCatalogCache.clear()
            val loadedAt = now()
            variableCatalogCache[cacheKey] = CachedVariableCatalog(catalog, loadedAt)
            variableCatalogRepository.save(
                connectionRestUrl = connection.restUrl,
                stored = StoredProcessVariableCatalog(
                    catalog = catalog,
                    loadedAtEpochMillis = loadedAt.toEpochMilli(),
                ),
            )
            catalog
        }
    }

    override fun invalidateProcessVariableCatalog(
        connection: CamundaConnection,
        processDefinitionKey: String,
    ) {
        val normalizedKey = processDefinitionKey.trim()
        variableCatalogCache.remove("${connection.restUrl}\n$normalizedKey")
        variableCatalogRepository.remove(connection.restUrl, normalizedKey)
    }

    override suspend fun loadProcessDefinitionDetails(
        connection: CamundaConnection,
        processDefinitionId: String,
        dateFilter: DashboardDateFilter,
    ): CamundaApiResult<ProcessDefinitionDetails> = withContext(Dispatchers.IO) {
        val pathId = encode(processDefinitionId)
        val queryId = encode(processDefinitionId)
        val definitionUrl = "${connection.restUrl}/process-definition/$pathId"
        val range = if (dateFilter.isLive) null else dateFilter.toCamundaDateRange()
        val instancesUrl = if (range == null) {
            "${connection.restUrl}/process-instance" +
                "?processDefinitionId=$queryId&sortBy=instanceId&sortOrder=desc&maxResults=$DETAIL_LIST_LIMIT"
        } else {
            "${connection.restUrl}/history/process-instance" +
                "?processDefinitionId=$queryId&startedAfter=${encode(range.after)}&startedBefore=${encode(range.before)}" +
                "&sortBy=startTime&sortOrder=desc&maxResults=$DETAIL_LIST_LIMIT"
        }
        val incidentsUrl = if (range == null) {
            "${connection.restUrl}/incident?processDefinitionId=$queryId&maxResults=$DETAIL_LIST_LIMIT"
        } else {
            "${connection.restUrl}/history/incident" +
                "?processDefinitionId=$queryId&createTimeAfter=${encode(range.after)}&createTimeBefore=${encode(range.before)}" +
                "&sortBy=createTime&sortOrder=desc&maxResults=$DETAIL_LIST_LIMIT"
        }

        apiCall("загрузка процесса", definitionUrl) {
            val definitionJson = json.parseToJsonElement(send(get(definitionUrl))).jsonObject
            val definition = parseDefinition(definitionJson)
            val instances = parseInstances(send(get(instancesUrl)))
            val incidents = parseIncidents(send(get(incidentsUrl)))
            val incidentsByInstance = incidents.groupingBy(ProcessIncidentWithInstance::processInstanceId).eachCount()
            ProcessDefinitionDetails(
                definition = definition.copy(
                    instances = instances.size,
                    incidents = incidents.size,
                ),
                instances = instances.map { instance ->
                    RuntimeInstanceListItem(
                        instance = instance,
                        incidentCount = incidentsByInstance[instance.id] ?: 0,
                    )
                },
                metadata = definitionMetadata(definitionJson),
                dateFilter = dateFilter,
            )
        }
    }

    override suspend fun loadProcessInstanceDetails(
        connection: CamundaConnection,
        processInstanceId: String,
    ): CamundaApiResult<ProcessInstanceDetails> = withContext(Dispatchers.IO) {
        val pathId = encode(processInstanceId)
        val queryId = encode(processInstanceId)
        val instanceUrl = "${connection.restUrl}/process-instance/$pathId"
        apiCall("загрузка заявки", instanceUrl) {
            val instanceJson = json.parseToJsonElement(send(get(instanceUrl))).jsonObject
            val instance = parseInstance(instanceJson)
            coroutineScope {
                val variables = async {
                    parseVariables(send(get("${connection.restUrl}/process-instance/$pathId/variables?deserializeValues=false")))
                }
                val activities = async {
                    parseActivityTree(send(get("${connection.restUrl}/process-instance/$pathId/activity-instances")))
                }
                val activityHistory = async {
                    loadPaged(
                        baseUrl = "${connection.restUrl}/history/activity-instance" +
                            "?processInstanceId=$queryId&sortBy=startTime&sortOrder=asc",
                        parser = ::parseHistoricActivityInstances,
                    ).values.toActivityExecutionSummaries()
                }
                val incidents = async {
                    parseIncidents(send(get("${connection.restUrl}/incident?processInstanceId=$queryId&maxResults=$DETAIL_LIST_LIMIT")))
                        .map(ProcessIncidentWithInstance::incident)
                }
                val jobs = async {
                    parseJobs(send(get("${connection.restUrl}/job?processInstanceId=$queryId&withException=true&maxResults=$DETAIL_LIST_LIMIT")))
                }
                val externalTasks = async {
                    parseExternalTasks(send(get("${connection.restUrl}/external-task?processInstanceId=$queryId&maxResults=$DETAIL_LIST_LIMIT")))
                }
                val bpmn = async {
                    loadBpmn(connection, instance.definitionId)
                }
                val loadedBpmn = bpmn.await()
                val loadedIncidents = incidents.await()
                val loadedJobs = jobs.await()
                val loadedExternalTasks = externalTasks.await()
                val loadedActivityHistory = activityHistory.await()
                val currentActivities = buildCurrentActivities(
                    runtimeActivities = activities.await(),
                    activityHistory = loadedActivityHistory,
                    incidents = loadedIncidents,
                    jobs = loadedJobs,
                    externalTasks = loadedExternalTasks,
                    diagram = loadedBpmn.diagram,
                )
                ProcessInstanceDetails(
                    instance = instance,
                    variables = variables.await(),
                    activeActivities = currentActivities,
                    activityHistory = loadedActivityHistory,
                    incidents = loadedIncidents,
                    jobs = loadedJobs,
                    externalTasks = loadedExternalTasks,
                    diagram = loadedBpmn.diagram,
                    bpmnXml = loadedBpmn.xml,
                    metadata = instanceMetadata(instanceJson),
                )
            }
        }
    }

    override suspend fun teleportProcessInstance(
        connection: CamundaConnection,
        processInstanceId: String,
        request: TeleportRequest,
    ): CamundaApiResult<Unit> = mutate(
        operation = "телепорт заявки",
        url = "${connection.restUrl}/process-instance/${encode(processInstanceId)}/modification",
        method = HttpMethod.Post,
        body = buildJsonObject {
            put("skipCustomListeners", request.skipCustomListeners)
            put("skipIoMappings", request.skipIoMappings)
            put("annotation", request.annotation)
            put("instructions", buildJsonArray {
                add(buildJsonObject {
                    put("type", "cancel")
                    put("activityInstanceId", request.sourceActivityInstanceId)
                })
                add(buildJsonObject {
                    put("type", "startBeforeActivity")
                    put("activityId", request.targetActivityId)
                })
            })
        }.toString(),
    )

    override suspend fun setJobRetries(
        connection: CamundaConnection,
        jobId: String,
        retries: Int,
    ): CamundaApiResult<Unit> = mutate(
        operation = "установка retries job",
        url = "${connection.restUrl}/job/${encode(jobId)}/retries",
        method = HttpMethod.Put,
        body = buildJsonObject { put("retries", retries) }.toString(),
    )

    override suspend fun setExternalTaskRetries(
        connection: CamundaConnection,
        externalTaskId: String,
        retries: Int,
    ): CamundaApiResult<Unit> = mutate(
        operation = "установка retries external task",
        url = "${connection.restUrl}/external-task/${encode(externalTaskId)}/retries",
        method = HttpMethod.Put,
        body = buildJsonObject { put("retries", retries) }.toString(),
    )

    override suspend fun unlockExternalTask(
        connection: CamundaConnection,
        externalTaskId: String,
    ): CamundaApiResult<Unit> = mutate(
        operation = "разблокировка external task",
        url = "${connection.restUrl}/external-task/${encode(externalTaskId)}/unlock",
        method = HttpMethod.Post,
        body = null,
    )

    override suspend fun setProcessInstanceSuspended(
        connection: CamundaConnection,
        processInstanceId: String,
        suspended: Boolean,
    ): CamundaApiResult<Unit> = mutate(
        operation = if (suspended) "приостановка заявки" else "активация заявки",
        url = "${connection.restUrl}/process-instance/${encode(processInstanceId)}/suspended",
        method = HttpMethod.Put,
        body = buildJsonObject { put("suspended", suspended) }.toString(),
    )

    override suspend fun updateProcessVariable(
        connection: CamundaConnection,
        processInstanceId: String,
        update: ProcessVariableUpdate,
    ): CamundaApiResult<Unit> {
        val url = "${connection.restUrl}/process-instance/${encode(processInstanceId)}" +
            "/variables/${encode(update.name)}"
        return try {
            val value = update.toJsonValue()
            val valueInfo = update.valueInfo
                ?.let { json.parseToJsonElement(it) as? JsonObject }
                ?: buildJsonObject { }
            mutate(
                operation = "изменение переменной ${update.name}",
                url = url,
                method = HttpMethod.Put,
                body = buildJsonObject {
                    put("value", value)
                    put("type", update.type)
                    put("valueInfo", valueInfo)
                }.toString(),
            )
        } catch (error: IllegalArgumentException) {
            CamundaApiResult.Failure(error.message ?: "Некорректное значение переменной.")
        }
    }

    private suspend fun mutate(
        operation: String,
        url: String,
        method: HttpMethod,
        body: String?,
    ): CamundaApiResult<Unit> = withContext(Dispatchers.IO) {
        apiCall(operation, url) {
            val request = when (method) {
                HttpMethod.Post -> post(url, body)
                HttpMethod.Put -> put(url, body.orEmpty())
            }
            send(request)
            Unit
        }
    }

    private fun get(url: String): HttpRequest = HttpRequest.newBuilder(URI(url))
        .timeout(REQUEST_TIMEOUT)
        .header("Accept", "application/json")
        .GET()
        .build()

    private fun post(url: String, body: String?): HttpRequest = HttpRequest.newBuilder(URI(url))
        .timeout(REQUEST_TIMEOUT)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .POST(body?.let(HttpRequest.BodyPublishers::ofString) ?: HttpRequest.BodyPublishers.noBody())
        .build()

    private fun put(url: String, body: String): HttpRequest = HttpRequest.newBuilder(URI(url))
        .timeout(REQUEST_TIMEOUT)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .PUT(HttpRequest.BodyPublishers.ofString(body))
        .build()

    private fun send(request: HttpRequest): String {
        val response = client.send(request, HttpResponse.BodyHandlers.ofString())
        if (response.statusCode() !in 200..299) {
            throw CamundaHttpException(
                status = response.statusCode(),
                serverMessage = extractServerMessage(response.body()),
                requestUrl = request.uri().toString(),
            )
        }
        return response.body()
    }

    private fun parseDefinitions(body: String): List<ProcessDefinitionSummary> = json
        .parseToJsonElement(body)
        .jsonArray
        .map { element -> parseDefinition(element.jsonObject) }

    private fun parseDefinition(value: JsonObject): ProcessDefinitionSummary = ProcessDefinitionSummary(
        id = requireNotNull(value.string("id")) { "Camunda не вернула ID process definition" },
        key = requireNotNull(value.string("key")) { "Camunda не вернула key process definition" },
        name = value.string("name"),
        version = value.int("version"),
        tenantId = value.string("tenantId"),
        suspended = value.boolean("suspended"),
        deploymentId = value.string("deploymentId"),
        instances = 0,
        incidents = 0,
        versionTag = value.string("versionTag"),
        category = value.string("category"),
        description = value.string("description"),
        resource = value.string("resource"),
        diagramResource = value.string("diagram"),
        historyTimeToLive = value.nullableInt("historyTimeToLive"),
        startableInTasklist = value["startableInTasklist"]?.jsonPrimitive?.booleanOrNull ?: true,
    )

    private fun parseStatistics(body: String): Map<String, DefinitionStatistics> = json
        .parseToJsonElement(body)
        .jsonArray
        .mapNotNull { element ->
            val value = element.jsonObject
            val id = value.string("id") ?: return@mapNotNull null
            val incidents = value["incidents"]
                ?.asArrayOrEmpty()
                ?.sumOf { incident -> incident.jsonObject.int("incidentCount") }
                ?: 0
            id to DefinitionStatistics(
                instances = value.int("instances"),
                incidents = incidents,
            )
        }
        .toMap()

    private fun parseInstances(body: String): List<ProcessInstanceSummary> = json
        .parseToJsonElement(body)
        .jsonArray
        .mapNotNull { element ->
            val value = element.jsonObject
            runCatching { parseInstance(element.jsonObject) }.getOrNull()
        }

    private fun parseInstance(value: JsonObject): ProcessInstanceSummary {
        val id = requireNotNull(value.string("id")) { "Camunda не вернула ID process instance" }
        val definitionId = requireNotNull(value.string("definitionId") ?: value.string("processDefinitionId")) {
            "Camunda не вернула definitionId"
        }
        val state = value.string("state")
        val endTime = value.string("endTime")
        return ProcessInstanceSummary(
            id = id,
            definitionId = definitionId,
            definitionKey = value.string("definitionKey")
                ?: value.string("processDefinitionKey")
                ?: definitionId.substringBefore(':'),
            businessKey = value.string("businessKey"),
            suspended = value.boolean("suspended"),
            tenantId = value.string("tenantId"),
            ended = value.boolean("ended") || endTime != null || state in ENDED_HISTORY_STATES,
            startTime = value.string("startTime"),
            endTime = endTime,
            state = state,
        )
    }

    private fun definitionMetadata(value: JsonObject): List<ProcessMetadataField> = listOf(
        metadataField("ID определения", value, "id"),
        metadataField("Ключ", value, "key"),
        metadataField("Название", value, "name"),
        metadataField("Описание", value, "description"),
        metadataField("Версия", value, "version"),
        metadataField("Метка версии", value, "versionTag"),
        metadataField("Категория", value, "category"),
        metadataField("Ресурс BPMN", value, "resource"),
        metadataField("Ресурс диаграммы", value, "diagram"),
        metadataField("ID развёртывания", value, "deploymentId"),
        metadataField("ID тенанта", value, "tenantId"),
        metadataField("Срок хранения истории", value, "historyTimeToLive"),
        metadataField("Доступен для запуска в списке задач", value, "startableInTasklist"),
        metadataField("Приостановлен", value, "suspended"),
    )

    private fun instanceMetadata(value: JsonObject): List<ProcessMetadataField> = listOf(
        metadataField("ID экземпляра", value, "id"),
        metadataField("ID определения", value, "definitionId"),
        metadataField("Ключ определения", value, "definitionKey"),
        metadataField("Бизнес-ключ", value, "businessKey"),
        metadataField("ID экземпляра кейса", value, "caseInstanceId"),
        metadataField("ID тенанта", value, "tenantId"),
        metadataField("Завершён", value, "ended"),
        metadataField("Приостановлен", value, "suspended"),
    )

    private fun metadataField(label: String, value: JsonObject, key: String): ProcessMetadataField =
        ProcessMetadataField(
            label = label,
            value = value[key]?.jsonPrimitive?.contentOrNull ?: "—",
        )

    private fun parseVariables(body: String): List<ProcessVariable> = json
        .parseToJsonElement(body)
        .jsonObject
        .map { (name, element) ->
            val value = element.jsonObject
            val rawValue = value["value"]
            ProcessVariable(
                name = name,
                type = value.string("type") ?: "Неизвестный",
                value = when (rawValue) {
                    is JsonPrimitive -> rawValue.contentOrNull ?: "null"
                    null -> "null"
                    else -> rawValue.toString()
                },
                valueInfo = value["valueInfo"]?.takeUnless { it.toString() == "{}" }?.toString(),
            )
        }
        .sortedBy { it.name.lowercase() }

    private fun parseVariableDescriptors(body: String): List<VariableDescriptorSample> = json
        .parseToJsonElement(body)
        .jsonArray
        .mapNotNull { element ->
            val value = element.jsonObject
            VariableDescriptorSample(
                name = value.string("name") ?: return@mapNotNull null,
                type = value.string("type") ?: "Неизвестный",
            )
        }

    private fun parseActivityTree(body: String): List<ActiveActivityInstance> {
        val result = mutableListOf<ActiveActivityInstance>()
        fun visitActivity(value: JsonObject, root: Boolean) {
            val type = value.string("activityType") ?: "unknown"
            if (!root && type != "processDefinition") {
                result += ActiveActivityInstance(
                    id = value.string("id").orEmpty(),
                    activityId = value.string("activityId").orEmpty(),
                    activityName = value.string("activityName") ?: value.string("name"),
                    activityType = type,
                    executionIds = value.stringArray("executionIds"),
                    incidentIds = value.stringArray("incidentIds"),
                )
            }
            value["childActivityInstances"]?.asArrayOrEmpty()?.forEach { child ->
                visitActivity(child.jsonObject, false)
            }
            value["childTransitionInstances"]?.asArrayOrEmpty()?.forEach { child ->
                val transition = child.jsonObject
                result += ActiveActivityInstance(
                    id = transition.string("id").orEmpty(),
                    activityId = transition.string("activityId").orEmpty(),
                    activityName = transition.string("activityName") ?: transition.string("name"),
                    activityType = transition.string("activityType") ?: "transition",
                    executionIds = listOfNotNull(transition.string("executionId")),
                    incidentIds = transition.stringArray("incidentIds"),
                    evidence = setOf(CurrentActivityEvidence.Transition),
                    cancellable = false,
                )
            }
        }
        visitActivity(json.parseToJsonElement(body).jsonObject, true)
        return result
    }

    private fun buildCurrentActivities(
        runtimeActivities: List<ActiveActivityInstance>,
        activityHistory: List<ActivityExecutionSummary>,
        incidents: List<ProcessIncident>,
        jobs: List<ProcessJob>,
        externalTasks: List<ProcessExternalTask>,
        diagram: BpmnDiagram,
    ): List<ActiveActivityInstance> {
        val evidenceByActivityId = mutableMapOf<String, MutableSet<CurrentActivityEvidence>>()
        fun evidence(activityId: String?, source: CurrentActivityEvidence) {
            activityId?.takeIf(String::isNotBlank)?.let {
                evidenceByActivityId.getOrPut(it, ::mutableSetOf) += source
            }
        }
        incidents.forEach { evidence(it.failedActivityId ?: it.activityId, CurrentActivityEvidence.Incident) }
        jobs.forEach { evidence(it.failedActivityId, CurrentActivityEvidence.Job) }
        externalTasks.forEach { evidence(it.activityId, CurrentActivityEvidence.ExternalTask) }

        val runtimeActivityIds = runtimeActivities.mapTo(mutableSetOf(), ActiveActivityInstance::activityId)
        activityHistory
            .filter { it.activeCount > 0 && it.activityId !in runtimeActivityIds }
            .forEach { evidence(it.activityId, CurrentActivityEvidence.UnfinishedHistory) }
        val enrichedRuntime = runtimeActivities.map { activity ->
            activity.copy(evidence = activity.evidence + evidenceByActivityId[activity.activityId].orEmpty())
        }
        val inferred = evidenceByActivityId
            .filterKeys { it !in runtimeActivityIds }
            .map { (activityId, evidence) ->
                val node = diagram.nodes.firstOrNull { it.id == activityId }
                ActiveActivityInstance(
                    id = "diagnostic:$activityId",
                    activityId = activityId,
                    activityName = node?.name,
                    activityType = node?.type ?: "unknown",
                    executionIds = emptyList(),
                    incidentIds = incidents
                        .filter { (it.failedActivityId ?: it.activityId) == activityId }
                        .map(ProcessIncident::id),
                    evidence = evidence,
                    cancellable = false,
                )
            }
        return (enrichedRuntime + inferred)
            .sortedWith(compareBy<ActiveActivityInstance> { it.activityName?.lowercase() ?: it.activityId.lowercase() }.thenBy { it.id })
    }

    private fun parseHistoricActivityInstances(body: String): List<HistoricActivityInstance> = json
        .parseToJsonElement(body)
        .jsonArray
        .mapNotNull { element ->
            val value = element.jsonObject
            HistoricActivityInstance(
                id = value.string("id") ?: return@mapNotNull null,
                activityId = value.string("activityId") ?: return@mapNotNull null,
                activityName = value.string("activityName"),
                activityType = value.string("activityType") ?: "unknown",
                processDefinitionId = value.string("processDefinitionId"),
                processInstanceId = value.string("processInstanceId"),
                parentActivityInstanceId = value.string("parentActivityInstanceId"),
                startTime = value.string("startTime"),
                endTime = value.string("endTime"),
                canceled = value.boolean("canceled"),
            )
        }

    private fun List<HistoricActivityInstance>.toActivityExecutionSummaries(): List<ActivityExecutionSummary> =
        asSequence()
            .filter { it.activityType != "processDefinition" }
            .groupBy(HistoricActivityInstance::activityId)
            .map { (activityId, executions) ->
                val representative = executions.last()
                ActivityExecutionSummary(
                    activityId = activityId,
                    activityName = executions.lastOrNull { !it.activityName.isNullOrBlank() }?.activityName,
                    activityType = representative.activityType,
                    completedCount = executions.count { it.endTime != null && !it.canceled },
                    activeCount = executions.count { it.endTime == null },
                    canceledCount = executions.count(HistoricActivityInstance::canceled),
                )
            }
            .sortedBy { it.activityName?.lowercase() ?: it.activityId.lowercase() }

    private fun parseIncidents(body: String): List<ProcessIncidentWithInstance> = json
        .parseToJsonElement(body)
        .jsonArray
        .mapNotNull { element ->
            val value = element.jsonObject
            val id = value.string("id") ?: return@mapNotNull null
            ProcessIncidentWithInstance(
                processInstanceId = value.string("processInstanceId").orEmpty(),
                incident = ProcessIncident(
                    id = id,
                    type = value.string("incidentType") ?: "unknown",
                    activityId = value.string("activityId"),
                    failedActivityId = value.string("failedActivityId"),
                    message = value.string("incidentMessage"),
                    timestamp = value.string("incidentTimestamp") ?: value.string("createTime"),
                    configuration = value.string("configuration"),
                    processDefinitionId = value.string("processDefinitionId"),
                    processInstanceId = value.string("processInstanceId"),
                    executionId = value.string("executionId"),
                    tenantId = value.string("tenantId"),
                    annotation = value.string("annotation"),
                ),
            )
        }

    private fun parseJobs(body: String): List<ProcessJob> = json
        .parseToJsonElement(body)
        .jsonArray
        .mapNotNull { element ->
            val value = element.jsonObject
            ProcessJob(
                id = value.string("id") ?: return@mapNotNull null,
                retries = value.int("retries"),
                failedActivityId = value.string("failedActivityId"),
                exceptionMessage = value.string("exceptionMessage"),
                dueDate = value.string("dueDate"),
                suspended = value.boolean("suspended"),
            )
        }

    private fun parseExternalTasks(body: String): List<ProcessExternalTask> = json
        .parseToJsonElement(body)
        .jsonArray
        .mapNotNull { element ->
            val value = element.jsonObject
            ProcessExternalTask(
                id = value.string("id") ?: return@mapNotNull null,
                activityId = value.string("activityId").orEmpty(),
                topicName = value.string("topicName").orEmpty(),
                retries = value.nullableInt("retries"),
                errorMessage = value.string("errorMessage"),
                workerId = value.string("workerId"),
                lockExpirationTime = value.string("lockExpirationTime"),
                suspended = value.boolean("suspended"),
            )
        }

    private fun loadBpmn(connection: CamundaConnection, processDefinitionId: String): LoadedBpmn {
        val cacheKey = "${connection.restUrl}\n$processDefinitionId"
        bpmnCache[cacheKey]?.let { return it }
        val url = "${connection.restUrl}/process-definition/${encode(processDefinitionId)}/xml"
        val xml = json.parseToJsonElement(send(get(url))).jsonObject.string("bpmn20Xml").orEmpty()
        val loaded = LoadedBpmn(xml = xml, diagram = bpmnXmlParser.parse(xml))
        if (bpmnCache.size >= BPMN_CACHE_LIMIT) bpmnCache.clear()
        bpmnCache[cacheKey] = loaded
        return loaded
    }

    private suspend fun loadIncidentActivities(
        connection: CamundaConnection,
        incidents: List<ProcessIncident>,
    ): List<DashboardIncidentActivity> {
        val aggregated = incidents
            .mapNotNull { incident ->
                (incident.failedActivityId ?: incident.activityId)?.let { activityId ->
                    (activityId to incident.processDefinitionId)
                }
            }
            .groupingBy { it }
            .eachCount()
            .entries
            .sortedByDescending { it.value }
            .take(DASHBOARD_ACTIVITY_LIMIT)
        val definitionIds = aggregated.mapNotNull { it.key.second }.distinct()
        val diagrams = coroutineScope {
            val semaphore = Semaphore(MAX_PARALLEL_HISTORY_REQUESTS)
            definitionIds.map { definitionId ->
                async {
                    semaphore.withPermit {
                        definitionId to runCatching { loadBpmn(connection, definitionId).diagram }.getOrNull()
                    }
                }
            }.awaitAll().toMap()
        }
        return aggregated.map { (activity, count) ->
            val node = activity.second?.let(diagrams::get)?.nodes?.firstOrNull { it.id == activity.first }
            DashboardIncidentActivity(
                activityId = activity.first,
                processDefinitionId = activity.second,
                count = count,
                activityName = node?.name,
                topic = node?.topic,
            )
        }
    }

    private fun loadUnfinishedActivities(connection: CamundaConnection): PagedResult<HistoricActivityInstance> = loadPaged(
        baseUrl = "${connection.restUrl}/history/activity-instance?unfinished=true&sortBy=startTime&sortOrder=asc",
        parser = ::parseHistoricActivityInstances,
    )

    private suspend fun loadLongestWaitingActivities(
        connection: CamundaConnection,
        activities: List<HistoricActivityInstance>,
    ): List<DashboardWaitingActivity> {
        val parentIds = activities.mapNotNull(HistoricActivityInstance::parentActivityInstanceId).toSet()
        val current = now()
        val aggregated = activities
            .asSequence()
            .filter { it.id !in parentIds && it.activityType != "processDefinition" }
            .mapNotNull { activity ->
                val definitionId = activity.processDefinitionId ?: return@mapNotNull null
                val processInstanceId = activity.processInstanceId ?: return@mapNotNull null
                val start = parseCamundaDateTime(activity.startTime)?.toInstant() ?: return@mapNotNull null
                WaitingActivitySample(
                    activity = activity,
                    processDefinitionId = definitionId,
                    processInstanceId = processInstanceId,
                    waitingMillis = Duration.between(start, current).toMillis().coerceAtLeast(0),
                )
            }
            .groupBy { it.processDefinitionId to it.activity.activityId }
            .map { (key, samples) ->
                val oldest = samples.maxBy(WaitingActivitySample::waitingMillis)
                DashboardWaitingActivity(
                    activityId = key.second,
                    activityName = samples.firstNotNullOfOrNull { it.activity.activityName?.takeIf(String::isNotBlank) },
                    activityType = oldest.activity.activityType,
                    topic = null,
                    processDefinitionId = key.first,
                    oldestProcessInstanceId = oldest.processInstanceId,
                    instanceCount = samples.map(WaitingActivitySample::processInstanceId).distinct().size,
                    averageWaitingMillis = samples.map(WaitingActivitySample::waitingMillis).average().toLong(),
                    maximumWaitingMillis = oldest.waitingMillis,
                    oldestStartedAt = oldest.activity.startTime,
                )
            }
            .sortedByDescending(DashboardWaitingActivity::maximumWaitingMillis)
            .take(DASHBOARD_WAITING_ACTIVITY_LIMIT)

        val diagrams = loadDiagrams(connection, aggregated.map(DashboardWaitingActivity::processDefinitionId))
        return aggregated.map { activity ->
            val node = diagrams[activity.processDefinitionId]?.nodes?.firstOrNull { it.id == activity.activityId }
            activity.copy(
                activityName = activity.activityName ?: node?.name,
                topic = node?.topic,
            )
        }
    }

    private suspend fun loadDiagrams(
        connection: CamundaConnection,
        definitionIds: List<String>,
    ): Map<String, BpmnDiagram?> = coroutineScope {
        val semaphore = Semaphore(MAX_PARALLEL_HISTORY_REQUESTS)
        definitionIds.distinct().map { definitionId ->
            async {
                semaphore.withPermit {
                    definitionId to runCatching { loadBpmn(connection, definitionId).diagram }.getOrNull()
                }
            }
        }.awaitAll().toMap()
    }

    private suspend fun loadHistoricStatistics(
        connection: CamundaConnection,
        definitions: List<ProcessDefinitionSummary>,
        dateFilter: DashboardDateFilter,
    ): Map<String, DefinitionStatistics> {
        val range = dateFilter.toCamundaDateRange()
        val semaphore = Semaphore(MAX_PARALLEL_HISTORY_REQUESTS)
        return coroutineScope {
            definitions.map { definition ->
                async {
                    semaphore.withPermit {
                        val definitionId = encode(definition.id)
                        val after = encode(range.after)
                        val before = encode(range.before)
                        val instancesUrl = "${connection.restUrl}/history/process-instance/count" +
                            "?processDefinitionId=$definitionId&startedAfter=$after&startedBefore=$before"
                        val incidentsUrl = "${connection.restUrl}/history/incident/count" +
                            "?processDefinitionId=$definitionId&createTimeAfter=$after&createTimeBefore=$before"
                        definition.id to DefinitionStatistics(
                            instances = parseCount(send(get(instancesUrl))),
                            incidents = parseCount(send(get(incidentsUrl))),
                        )
                    }
                }
            }.awaitAll().toMap()
        }
    }

    private fun loadHistoricInstances(
        connection: CamundaConnection,
        range: CamundaDateRange,
        completed: Boolean,
    ): PagedResult<ProcessInstanceSummary> {
        val timeFilter = if (completed) {
            "completed=true&finishedAfter=${encode(range.after)}&finishedBefore=${encode(range.before)}" +
                "&sortBy=endTime"
        } else {
            "startedAfter=${encode(range.after)}&startedBefore=${encode(range.before)}&sortBy=startTime"
        }
        return loadPaged(
            baseUrl = "${connection.restUrl}/history/process-instance?$timeFilter&sortOrder=asc",
            parser = ::parseInstances,
        )
    }

    private fun loadHistoricIncidents(
        connection: CamundaConnection,
        range: CamundaDateRange,
        processDefinitionId: String? = null,
        processDefinitionKey: String? = null,
    ): PagedResult<ProcessIncident> {
        val definitionFilter = processDefinitionId?.let { "&processDefinitionId=${encode(it)}" }.orEmpty()
        val keyFilter = processDefinitionKey?.let { "&processDefinitionKeyIn=${encode(it)}" }.orEmpty()
        return loadPaged(
            baseUrl = "${connection.restUrl}/history/incident" +
                "?createTimeAfter=${encode(range.after)}&createTimeBefore=${encode(range.before)}" +
                "$definitionFilter$keyFilter&sortBy=createTime&sortOrder=asc",
            parser = { body -> parseIncidents(body).map(ProcessIncidentWithInstance::incident) },
        )
    }

    private fun loadRuntimeIncidents(
        connection: CamundaConnection,
        processDefinitionId: String?,
        processDefinitionKey: String? = null,
    ): List<ProcessIncident> {
        val filters = listOfNotNull(
            processDefinitionId?.let { "processDefinitionId=${encode(it)}" },
            processDefinitionKey?.let { "processDefinitionKeyIn=${encode(it)}" },
        )
        val filter = filters.takeIf { it.isNotEmpty() }?.joinToString(prefix = "?", separator = "&").orEmpty()
        return loadPaged(
            baseUrl = "${connection.restUrl}/incident$filter",
            parser = { body -> parseIncidents(body).map(ProcessIncidentWithInstance::incident) },
        ).values
    }

    private fun <T> loadPaged(baseUrl: String, parser: (String) -> List<T>): PagedResult<T> {
        val values = mutableListOf<T>()
        var firstResult = 0
        while (values.size < HISTORY_CHART_LIMIT) {
            val separator = if ('?' in baseUrl) '&' else '?'
            val pageUrl = "$baseUrl${separator}firstResult=$firstResult&maxResults=$HISTORY_PAGE_SIZE"
            val page = parser(send(get(pageUrl)))
            values += page
            if (page.size < HISTORY_PAGE_SIZE) return PagedResult(values, truncated = false)
            firstResult += page.size
        }
        return PagedResult(values.take(HISTORY_CHART_LIMIT), truncated = true)
    }

    private fun buildTimeline(
        range: CamundaDateRange,
        started: List<ProcessInstanceSummary>,
        completed: List<ProcessInstanceSummary>,
        incidents: List<ProcessIncident>,
    ): List<DashboardTimelinePoint> {
        val hourly = ChronoUnit.HOURS.between(range.start, range.end) <= 48
        val formatter = if (hourly) DateTimeFormatter.ofPattern("dd.MM HH:mm") else DateTimeFormatter.ofPattern("dd.MM")
        val step: (ZonedDateTime) -> ZonedDateTime = if (hourly) {
            { it.plusHours(1) }
        } else {
            { it.plusDays(1) }
        }
        var cursor = if (hourly) range.start.truncatedTo(ChronoUnit.HOURS) else range.start.toLocalDate().atStartOfDay(range.start.zone)
        val points = mutableListOf<DashboardTimelinePoint>()
        while (cursor.isBefore(range.end)) {
            val bucketStart = cursor
            val bucketEnd = step(cursor)
            points += DashboardTimelinePoint(
                label = formatter.format(bucketStart),
                started = started.count { it.startTime.isInside(bucketStart, bucketEnd) },
                completed = completed.count { it.endTime.isInside(bucketStart, bucketEnd) },
                incidents = incidents.count { it.timestamp.isInside(bucketStart, bucketEnd) },
            )
            cursor = bucketEnd
        }
        return points
    }

    private fun String?.isInside(start: ZonedDateTime, end: ZonedDateTime): Boolean {
        val value = parseCamundaDateTime(this)?.toInstant() ?: return false
        return !value.isBefore(start.toInstant()) && value.isBefore(end.toInstant())
    }

    private fun durationMillis(instance: ProcessInstanceSummary): Long? {
        val start = parseCamundaDateTime(instance.startTime)?.toInstant() ?: return null
        val end = parseCamundaDateTime(instance.endTime)?.toInstant() ?: return null
        return Duration.between(start, end).toMillis().takeIf { it >= 0 }
    }

    private fun buildDurationBuckets(durations: List<Long>): List<DashboardDurationBucket> {
        val minute = 60_000L
        val hour = 60 * minute
        return listOf(
            DashboardDurationBucket("< 1 мин", durations.count { it < minute }),
            DashboardDurationBucket("1–5 мин", durations.count { it in minute until 5 * minute }),
            DashboardDurationBucket("5–15 мин", durations.count { it in 5 * minute until 15 * minute }),
            DashboardDurationBucket("15–60 мин", durations.count { it in 15 * minute until hour }),
            DashboardDurationBucket("1–4 ч", durations.count { it in hour until 4 * hour }),
            DashboardDurationBucket("4–24 ч", durations.count { it in 4 * hour until 24 * hour }),
            DashboardDurationBucket("> 24 ч", durations.count { it >= 24 * hour }),
        )
    }

    private fun List<Long>.percentile(value: Double): Long? {
        if (isEmpty()) return null
        val position = lastIndex * value.coerceIn(0.0, 1.0)
        val lowerIndex = position.toInt()
        val upperIndex = (lowerIndex + 1).coerceAtMost(lastIndex)
        val fraction = position - lowerIndex
        return this[lowerIndex] + ((this[upperIndex] - this[lowerIndex]) * fraction).toLong()
    }

    private fun parseCamundaDateTime(value: String?): OffsetDateTime? {
        if (value.isNullOrBlank()) return null
        return runCatching { OffsetDateTime.parse(value, CAMUNDA_DATE_FORMAT) }
            .recoverCatching { OffsetDateTime.parse(value, DateTimeFormatter.ISO_OFFSET_DATE_TIME) }
            .getOrNull()
    }

    private fun DashboardDateFilter.toDashboardRange(): CamundaDateRange = if (isLive) {
        val end = ZonedDateTime.now(ZoneId.systemDefault())
        val start = end.minusHours(24)
        CamundaDateRange(
            after = CAMUNDA_DATE_FORMAT.format(start),
            before = CAMUNDA_DATE_FORMAT.format(end),
            start = start,
            end = end,
        )
    } else {
        toCamundaDateRange()
    }

    private fun DashboardDateFilter.timelineTitle(): String = when (preset) {
        DashboardDatePreset.Live -> "Динамика за последние 24 часа"
        DashboardDatePreset.Today -> "Динамика за сегодня"
        DashboardDatePreset.Yesterday -> "Динамика за вчера"
        DashboardDatePreset.Custom -> "Динамика за период $fromDate — $toDate"
    }

    private fun DashboardDateFilter.toCamundaDateRange(): CamundaDateRange {
        val today = LocalDate.now(ZoneId.systemDefault())
        val (startDate, endDate) = when (preset) {
            DashboardDatePreset.Live -> error("Для текущей статистики диапазон дат не используется")
            DashboardDatePreset.Today -> today to today
            DashboardDatePreset.Yesterday -> today.minusDays(1) to today.minusDays(1)
            DashboardDatePreset.Custom -> {
                val start = parseDate(fromDate)
                val end = parseDate(toDate)
                require(!start.isAfter(end)) { "Начальная дата не может быть позже конечной." }
                start to end
            }
        }
        val zone = ZoneId.systemDefault()
        val after = startDate.atStartOfDay(zone).minusNanos(1_000_000)
        val before = endDate.plusDays(1).atStartOfDay(zone)
        return CamundaDateRange(
            after = CAMUNDA_DATE_FORMAT.format(after),
            before = CAMUNDA_DATE_FORMAT.format(before),
            start = startDate.atStartOfDay(zone),
            end = endDate.plusDays(1).atStartOfDay(zone),
        )
    }

    private fun parseDate(value: String): LocalDate = try {
        LocalDate.parse(value.trim(), DateTimeFormatter.ISO_LOCAL_DATE)
    } catch (_: DateTimeParseException) {
        throw IllegalArgumentException("Введите дату в формате ГГГГ-ММ-ДД.")
    }

    private fun parseCount(body: String): Int = json
        .parseToJsonElement(body)
        .jsonObject
        .int("count")

    private fun String.toSearchJsonValues(type: VariableValueType): List<JsonPrimitive> = when (type) {
        VariableValueType.Auto -> buildList {
            add(JsonPrimitive(this@toSearchJsonValues))
            trim().toLongOrNull()?.let { add(JsonPrimitive(it)) }
            trim().toDoubleOrNull()?.takeIf(Double::isFinite)?.let { number ->
                if (trim().toLongOrNull() == null) add(JsonPrimitive(number))
            }
            when (trim().lowercase()) {
                "true" -> add(JsonPrimitive(true))
                "false" -> add(JsonPrimitive(false))
            }
        }.distinctBy(JsonPrimitive::toString)
        VariableValueType.String -> listOf(JsonPrimitive(this))
        VariableValueType.Number -> listOf(
            trim().toLongOrNull()?.let(::JsonPrimitive)
                ?: trim().toDoubleOrNull()?.takeIf(Double::isFinite)?.let(::JsonPrimitive)
                ?: throw IllegalArgumentException("Значение переменной должно быть корректным числом."),
        )
        VariableValueType.Boolean -> listOf(
            when (trim().lowercase()) {
                "true" -> JsonPrimitive(true)
                "false" -> JsonPrimitive(false)
                else -> throw IllegalArgumentException("Логическое значение должно быть true или false.")
            },
        )
    }

    private fun ProcessVariableUpdate.toJsonValue(): JsonElement = when (type.lowercase()) {
        "boolean" -> when (value.trim().lowercase()) {
            "true" -> JsonPrimitive(true)
            "false" -> JsonPrimitive(false)
            else -> throw IllegalArgumentException("Для Boolean допустимы только true или false.")
        }
        "short" -> value.trim().toShortOrNull()?.let(::JsonPrimitive)
            ?: throw IllegalArgumentException("Значение должно быть целым числом Short.")
        "integer" -> value.trim().toIntOrNull()?.let(::JsonPrimitive)
            ?: throw IllegalArgumentException("Значение должно быть целым числом Integer.")
        "long" -> value.trim().toLongOrNull()?.let(::JsonPrimitive)
            ?: throw IllegalArgumentException("Значение должно быть целым числом Long.")
        "double" -> value.trim().toDoubleOrNull()?.takeIf(Double::isFinite)?.let(::JsonPrimitive)
            ?: throw IllegalArgumentException("Значение должно быть конечным числом Double.")
        "null" -> JsonNull
        "json" -> {
            runCatching { json.parseToJsonElement(value) }
                .getOrElse { throw IllegalArgumentException("Значение должно содержать корректный JSON.") }
            JsonPrimitive(value)
        }
        else -> JsonPrimitive(value)
    }

    private fun extractServerMessage(body: String): String? = runCatching {
        json.parseToJsonElement(body).jsonObject.string("message")
    }.getOrNull()

    private suspend inline fun <T> apiCall(
        operation: String,
        url: String,
        crossinline block: suspend () -> T,
    ): CamundaApiResult<T> = try {
        CamundaApiResult.Success(block())
    } catch (error: CancellationException) {
        throw error
    } catch (error: InterruptedException) {
        Thread.currentThread().interrupt()
        CamundaApiResult.Failure("Операция была прервана. Повторите попытку.")
    } catch (error: Throwable) {
        CamundaApiResult.Failure(error.toUserMessage(operation, url))
    }

    private fun Throwable.toUserMessage(operation: String, url: String): String {
        val causes = generateSequence(this) { it.cause }.toList()
        val message = when {
            this is CamundaHttpException -> when (status) {
                400 -> "Camunda отклонила запрос (HTTP 400). ${serverMessage.orEmpty()}"
                401 -> "Требуется авторизация (HTTP 401)."
                403 -> "Доступ запрещён (HTTP 403). У подключения недостаточно прав."
                404 -> "REST endpoint Camunda не найден (HTTP 404)."
                429 -> "Слишком много запросов (HTTP 429). Подождите и повторите попытку."
                in 500..599 -> "Camunda или gateway вернули HTTP $status. ${serverMessage.orEmpty()}"
                else -> "Camunda вернула HTTP $status. ${serverMessage.orEmpty()}"
            }
            causes.any { it is HttpTimeoutException } -> "Время ожидания истекло. Проверьте VPN и доступность Camunda."
            causes.any { it is UnknownHostException } -> "Хост Camunda не найден. Проверьте DNS, адрес и VPN."
            causes.any { it is SSLException } -> "Ошибка TLS. Проверьте HTTPS-сертификат."
            causes.any { it is ConnectException } -> "Подключение отклонено или хост недоступен. Проверьте VPN, хост и порт."
            this is IllegalArgumentException -> this.message.orEmpty().ifBlank { "Некорректное значение запроса." }
            causes.any { it is IOException } -> "Сетевая ошибка: ${causes.first { it is IOException }.message.orEmpty()}"
            else -> "Не удалось выполнить операцию: ${this.message.orEmpty().ifBlank { this::class.simpleName.orEmpty() }}"
        }.trim()
        val requestUrl = (this as? CamundaHttpException)?.requestUrl ?: url
        return "$message\nЗапрос: $requestUrl"
    }

    private data class DefinitionStatistics(
        val instances: Int = 0,
        val incidents: Int = 0,
    )

    private data class ProcessIncidentWithInstance(
        val processInstanceId: String,
        val incident: ProcessIncident,
    )

    private data class HistoricActivityInstance(
        val id: String,
        val activityId: String,
        val activityName: String?,
        val activityType: String,
        val processDefinitionId: String?,
        val processInstanceId: String?,
        val parentActivityInstanceId: String?,
        val startTime: String?,
        val endTime: String?,
        val canceled: Boolean,
    )

    private data class WaitingActivitySample(
        val activity: HistoricActivityInstance,
        val processDefinitionId: String,
        val processInstanceId: String,
        val waitingMillis: Long,
    )

    private data class LoadedBpmn(
        val xml: String,
        val diagram: BpmnDiagram,
    )

    private data class VariableDescriptorSample(
        val name: String,
        val type: String,
    )

    private data class CachedVariableCatalog(
        val catalog: ProcessVariableCatalog,
        val loadedAt: Instant,
    )

    private enum class HttpMethod { Post, Put }

    private data class CamundaDateRange(
        val after: String,
        val before: String,
        val start: ZonedDateTime,
        val end: ZonedDateTime,
    )

    private data class PagedResult<T>(
        val values: List<T>,
        val truncated: Boolean,
    )

    private class CamundaHttpException(
        val status: Int,
        val serverMessage: String?,
        val requestUrl: String,
    ) : RuntimeException()

    private companion object {
        val CONNECT_TIMEOUT: Duration = Duration.ofSeconds(5)
        val REQUEST_TIMEOUT: Duration = Duration.ofSeconds(20)
        const val SEARCH_LIMIT = 100
        const val SEARCH_DEFINITION_LIMIT = 500
        const val DETAIL_LIST_LIMIT = 500
        const val BPMN_CACHE_LIMIT = 16
        const val VARIABLE_CATALOG_CACHE_LIMIT = 64
        const val VARIABLE_CATALOG_INSTANCE_LIMIT = SEARCH_LIMIT
        const val VARIABLE_INSTANCE_ID_BATCH_SIZE = 40
        const val MAX_PARALLEL_HISTORY_REQUESTS = 6
        const val HISTORY_PAGE_SIZE = 1_000
        const val HISTORY_CHART_LIMIT = 20_000
        const val DASHBOARD_ACTIVITY_LIMIT = 12
        const val DASHBOARD_WAITING_ACTIVITY_LIMIT = 6
        val ENDED_HISTORY_STATES = setOf("COMPLETED", "EXTERNALLY_TERMINATED", "INTERNALLY_TERMINATED")
        val VARIABLE_CATALOG_CACHE_TTL: Duration = Duration.ofHours(24)
        val CAMUNDA_DATE_FORMAT: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss.SSSZ")
        val json = Json { ignoreUnknownKeys = true }
    }
}

private fun encode(value: String): String = URLEncoder.encode(value, StandardCharsets.UTF_8)

private fun JsonObject.string(name: String): String? = this[name]
    ?.jsonPrimitive
    ?.contentOrNull

private fun JsonObject.int(name: String): Int = this[name]
    ?.jsonPrimitive
    ?.intOrNull
    ?: 0

private fun JsonObject.boolean(name: String): Boolean = this[name]
    ?.jsonPrimitive
    ?.booleanOrNull
    ?: false

private fun JsonObject.nullableInt(name: String): Int? = this[name]
    ?.jsonPrimitive
    ?.intOrNull

private fun JsonObject.stringArray(name: String): List<String> = this[name]
    ?.asArrayOrEmpty()
    ?.mapNotNull { it.jsonPrimitive.contentOrNull }
    .orEmpty()

private fun JsonElement.asArrayOrEmpty(): JsonArray = this as? JsonArray ?: JsonArray(emptyList())
