package com.malinatrash.camundasupport.model

enum class DeploymentSort(val label: String) {
    NewestFirst("Сначала новые"),
    OldestFirst("Сначала старые"),
}

enum class VariableValueType(val label: String) {
    Auto("Авто"),
    String("Строка"),
    Number("Число"),
    Boolean("Логическое"),
}

enum class DashboardDatePreset(val label: String) {
    Live("Сейчас"),
    Today("Сегодня"),
    Yesterday("Вчера"),
    Custom("Период"),
}

data class DashboardDateFilter(
    val preset: DashboardDatePreset = DashboardDatePreset.Live,
    val fromDate: String = "",
    val toDate: String = "",
) {
    val isLive: Boolean
        get() = preset == DashboardDatePreset.Live
}

data class ProcessDefinitionSummary(
    val id: String,
    val key: String,
    val name: String?,
    val version: Int,
    val tenantId: String?,
    val suspended: Boolean,
    val deploymentId: String?,
    val instances: Int,
    val incidents: Int,
    val versionTag: String? = null,
    val category: String? = null,
    val description: String? = null,
    val resource: String? = null,
    val diagramResource: String? = null,
    val historyTimeToLive: Int? = null,
    val startableInTasklist: Boolean = true,
) {
    val displayName: String
        get() = name?.takeIf(String::isNotBlank) ?: key
}

data class ProcessDashboard(
    val definitions: List<ProcessDefinitionSummary>,
    val timeline: List<DashboardTimelinePoint> = emptyList(),
    val incidentTypes: List<DashboardIncidentType> = emptyList(),
    val incidentActivities: List<DashboardIncidentActivity> = emptyList(),
    val longestWaitingActivities: List<DashboardWaitingActivity> = emptyList(),
    val durationBuckets: List<DashboardDurationBucket> = emptyList(),
    val timelineTitle: String = "Динамика заявок",
    val averageDurationMillis: Long? = null,
    val medianDurationMillis: Long? = null,
    val p95DurationMillis: Long? = null,
    val completedInstances: Int = 0,
    val timelineTruncated: Boolean = false,
) {
    val instances: Int
        get() = definitions.sumOf { it.instances }

    val incidents: Int
        get() = definitions.sumOf { it.incidents }

    val suspendedDefinitions: Int
        get() = definitions.count { it.suspended }

    val affectedDefinitions: Int
        get() = definitions.count { it.incidents > 0 }

    val incidentRatePercent: Double
        get() = if (instances == 0) 0.0 else incidents * 100.0 / instances
}

data class DashboardTimelinePoint(
    val label: String,
    val started: Int,
    val completed: Int,
    val incidents: Int,
)

data class DashboardIncidentType(
    val type: String,
    val count: Int,
)

data class DashboardIncidentActivity(
    val activityId: String,
    val processDefinitionId: String?,
    val count: Int,
    val activityName: String? = null,
    val topic: String? = null,
)

data class DashboardDurationBucket(
    val label: String,
    val count: Int,
)

data class DashboardWaitingActivity(
    val activityId: String,
    val activityName: String?,
    val activityType: String,
    val topic: String?,
    val processDefinitionId: String,
    val oldestProcessInstanceId: String,
    val instanceCount: Int,
    val averageWaitingMillis: Long,
    val maximumWaitingMillis: Long,
    val oldestStartedAt: String?,
)

data class ProcessInstanceSummary(
    val id: String,
    val definitionId: String,
    val definitionKey: String,
    val businessKey: String?,
    val suspended: Boolean,
    val tenantId: String?,
    val ended: Boolean = false,
    val startTime: String? = null,
    val endTime: String? = null,
    val state: String? = null,
)
