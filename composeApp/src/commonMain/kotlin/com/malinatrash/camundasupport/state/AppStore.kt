package com.malinatrash.camundasupport.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.malinatrash.camundasupport.data.ConnectionRepository
import com.malinatrash.camundasupport.model.AppDestination
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.ConnectionDraft
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.ProcessInstanceDeepLink
import com.malinatrash.camundasupport.model.matchesOrigin
import com.malinatrash.camundasupport.model.toConnection

sealed interface DeepLinkNavigationResult {
    data object Opened : DeepLinkNavigationResult

    data class ConnectionNotFound(val link: ProcessInstanceDeepLink) : DeepLinkNavigationResult

    data class ConnectionChoiceRequired(
        val link: ProcessInstanceDeepLink,
        val connections: List<CamundaConnection>,
    ) : DeepLinkNavigationResult
}

class AppStore(
    private val connectionRepository: ConnectionRepository,
) {
    private var processInstanceReturnDestination = AppDestination.Processes
    val connections = mutableStateListOf<CamundaConnection>()

    var selectedConnectionId by mutableStateOf<String?>(null)
        private set

    var destination by mutableStateOf(AppDestination.Overview)
        private set

    var isConnectionDialogOpen by mutableStateOf(false)
        private set

    var processSearch by mutableStateOf("")
        private set

    var selectedProcessDefinitionId by mutableStateOf<String?>(null)
        private set

    var selectedProcessInstanceId by mutableStateOf<String?>(null)
        private set

    var selectedDashboardDateFilter by mutableStateOf(DashboardDateFilter())
        private set

    var selectedIncidentProcessDefinitionId by mutableStateOf<String?>(null)
        private set

    var selectedIncidentDateFilter by mutableStateOf(DashboardDateFilter())
        private set

    init {
        connections += connectionRepository.load().sortedBy { it.name.lowercase() }
        selectedConnectionId = connections.firstOrNull()?.id
    }

    val selectedConnection: CamundaConnection?
        get() = connections.firstOrNull { it.id == selectedConnectionId }

    fun selectConnection(connectionId: String) {
        selectedConnectionId = connectionId
        selectedProcessDefinitionId = null
        selectedProcessInstanceId = null
        destination = AppDestination.Overview
    }

    fun navigate(destination: AppDestination) {
        if (destination == AppDestination.Incidents) {
            selectedIncidentProcessDefinitionId = null
            selectedIncidentDateFilter = DashboardDateFilter()
        }
        this.destination = destination
    }

    fun openIncidents(processDefinitionId: String?, dateFilter: DashboardDateFilter) {
        selectedIncidentProcessDefinitionId = processDefinitionId
        selectedIncidentDateFilter = dateFilter
        destination = AppDestination.Incidents
    }

    fun openProcessDefinition(
        processDefinitionId: String,
        dateFilter: DashboardDateFilter = DashboardDateFilter(),
    ) {
        selectedProcessDefinitionId = processDefinitionId
        selectedDashboardDateFilter = dateFilter
        selectedProcessInstanceId = null
        destination = AppDestination.ProcessDefinition
    }

    fun openProcessInstance(processInstanceId: String) {
        processInstanceReturnDestination = destination
        selectedProcessInstanceId = processInstanceId
        destination = AppDestination.ProcessInstance
    }

    fun openDeepLink(link: ProcessInstanceDeepLink): DeepLinkNavigationResult {
        val matchingConnections = connections.filter { it.matchesOrigin(link.origin) }
        if (matchingConnections.isEmpty()) return DeepLinkNavigationResult.ConnectionNotFound(link)

        val target = matchingConnections.singleOrNull()
            ?: matchingConnections.firstOrNull { it.id == selectedConnectionId }
            ?: return DeepLinkNavigationResult.ConnectionChoiceRequired(link, matchingConnections)
        openDeepLinkWithConnection(target.id, link)
        return DeepLinkNavigationResult.Opened
    }

    fun openDeepLinkWithConnection(connectionId: String, link: ProcessInstanceDeepLink) {
        val connection = connections.firstOrNull { it.id == connectionId && it.matchesOrigin(link.origin) } ?: return
        selectConnection(connection.id)
        openProcessInstance(link.processInstanceId)
    }

    fun backFromProcessDefinition() {
        destination = AppDestination.Overview
    }

    fun backFromProcessInstance() {
        destination = if (
            processInstanceReturnDestination == AppDestination.ProcessDefinition && selectedProcessDefinitionId == null
        ) AppDestination.Overview else processInstanceReturnDestination
        selectedProcessInstanceId = null
    }

    fun openConnectionDialog() {
        isConnectionDialogOpen = true
    }

    fun closeConnectionDialog() {
        isConnectionDialogOpen = false
    }

    fun addConnection(draft: ConnectionDraft, engineVersion: String?) {
        val idBase = draft.name
            .trim()
            .lowercase()
            .map { char -> if (char.isLetterOrDigit()) char else '-' }
            .joinToString("")
            .trim('-')
            .ifBlank { "connection" }
        var id = idBase
        var suffix = 2
        while (connections.any { it.id == id }) {
            id = "$idBase-$suffix"
            suffix += 1
        }

        val connection = draft.toConnection(id, engineVersion)
        connectionRepository.save(connection)
        connections += connection
        selectedConnectionId = connection.id
        destination = AppDestination.Overview
        isConnectionDialogOpen = false
    }

    fun deleteConnection(connectionId: String) {
        connectionRepository.delete(connectionId)
        connections.removeAll { it.id == connectionId }
        if (selectedConnectionId == connectionId) {
            selectedConnectionId = connections.firstOrNull()?.id
        }
    }

    fun updateProcessSearch(value: String) {
        processSearch = value
    }
}
