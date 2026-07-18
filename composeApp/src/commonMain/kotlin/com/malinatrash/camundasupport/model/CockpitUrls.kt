package com.malinatrash.camundasupport.model

fun CamundaConnection.processDefinitionCockpitUrl(processDefinitionId: String): String? = cockpitRoute(
    "process-definition/$processDefinitionId",
)

fun CamundaConnection.processInstanceCockpitUrl(processInstanceId: String): String? = cockpitRoute(
    "process-instance/$processInstanceId/runtime",
)

fun CamundaConnection.cockpitDashboardUrl(): String? = cockpitRoute("dashboard")

private fun CamundaConnection.cockpitRoute(route: String): String? {
    val base = cockpitUrl.trim().trimEnd('/')
    if (base.isEmpty()) return null
    return "$base/default/#/$route"
}
