package com.malinatrash.camundasupport.model

data class ProcessDefinitionDetails(
    val definition: ProcessDefinitionSummary,
    val instances: List<RuntimeInstanceListItem>,
    val metadata: List<ProcessMetadataField>,
    val dateFilter: DashboardDateFilter,
)

data class RuntimeInstanceListItem(
    val instance: ProcessInstanceSummary,
    val incidentCount: Int,
)

data class ProcessInstanceDetails(
    val instance: ProcessInstanceSummary,
    val variables: List<ProcessVariable>,
    val activeActivities: List<ActiveActivityInstance>,
    val activityHistory: List<ActivityExecutionSummary>,
    val incidents: List<ProcessIncident>,
    val jobs: List<ProcessJob>,
    val externalTasks: List<ProcessExternalTask>,
    val diagram: BpmnDiagram,
    val bpmnXml: String,
    val metadata: List<ProcessMetadataField>,
)

data class ActivityExecutionSummary(
    val activityId: String,
    val activityName: String?,
    val activityType: String,
    val completedCount: Int,
    val activeCount: Int,
    val canceledCount: Int,
)

data class ProcessMetadataField(
    val label: String,
    val value: String,
)

data class ProcessVariable(
    val name: String,
    val type: String,
    val value: String,
    val valueInfo: String?,
)

data class ProcessVariableUpdate(
    val name: String,
    val type: String,
    val value: String,
    val valueInfo: String?,
)

data class ActiveActivityInstance(
    val id: String,
    val activityId: String,
    val activityName: String?,
    val activityType: String,
    val executionIds: List<String>,
    val incidentIds: List<String>,
)

data class ProcessIncident(
    val id: String,
    val type: String,
    val activityId: String?,
    val failedActivityId: String?,
    val message: String?,
    val timestamp: String?,
    val configuration: String?,
    val processDefinitionId: String? = null,
    val processInstanceId: String? = null,
    val executionId: String? = null,
    val tenantId: String? = null,
    val annotation: String? = null,
)

data class ProcessJob(
    val id: String,
    val retries: Int,
    val failedActivityId: String?,
    val exceptionMessage: String?,
    val dueDate: String?,
    val suspended: Boolean,
)

data class ProcessExternalTask(
    val id: String,
    val activityId: String,
    val topicName: String,
    val retries: Int?,
    val errorMessage: String?,
    val workerId: String?,
    val lockExpirationTime: String?,
    val suspended: Boolean,
)

data class BpmnDiagram(
    val nodes: List<BpmnNode>,
    val edges: List<BpmnEdge>,
) {
    val teleportTargets: List<BpmnNode>
        get() = nodes.filter { it.type in TELEPORT_TARGET_TYPES }

    private companion object {
        val TELEPORT_TARGET_TYPES = setOf(
            "task",
            "userTask",
            "serviceTask",
            "scriptTask",
            "manualTask",
            "businessRuleTask",
            "sendTask",
            "receiveTask",
            "callActivity",
            "subProcess",
            "intermediateCatchEvent",
            "intermediateThrowEvent",
            "exclusiveGateway",
            "parallelGateway",
            "inclusiveGateway",
            "eventBasedGateway",
        )
    }
}

data class BpmnNode(
    val id: String,
    val name: String?,
    val type: String,
    val x: Float,
    val y: Float,
    val width: Float,
    val height: Float,
    val topic: String?,
)

data class BpmnEdge(
    val id: String,
    val sourceRef: String?,
    val targetRef: String?,
    val waypoints: List<BpmnPoint>,
)

data class BpmnPoint(
    val x: Float,
    val y: Float,
)

data class TeleportRequest(
    val sourceActivityInstanceId: String,
    val targetActivityId: String,
    val annotation: String,
    val skipCustomListeners: Boolean = true,
    val skipIoMappings: Boolean = true,
)
