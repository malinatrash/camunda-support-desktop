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
import com.malinatrash.camundasupport.model.ProcessInstanceSummary
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

data class OpenProcessTab(
    val id: String,
    val connectionId: String,
    val connectionName: String,
    val processInstanceId: String,
    val businessKey: String? = null,
    val processDefinitionKey: String? = null,
    val returnDestination: AppDestination = AppDestination.Processes,
)

class AppStore(
    private val connectionRepository: ConnectionRepository,
) {
    private var processInstanceReturnDestination = AppDestination.Processes
    val connections = mutableStateListOf<CamundaConnection>()
    val openProcessTabs = mutableStateListOf<OpenProcessTab>()

    var activeProcessTabId by mutableStateOf<String?>(null)
        private set

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
        activeProcessTabId = null
        destination = AppDestination.Overview
    }

    fun navigate(destination: AppDestination) {
        if (destination == AppDestination.Incidents) {
            selectedIncidentProcessDefinitionId = null
            selectedIncidentDateFilter = DashboardDateFilter()
        }
        if (destination != AppDestination.ProcessInstance) {
            selectedProcessInstanceId = null
            activeProcessTabId = null
        }
        this.destination = destination
    }

    fun openIncidents(processDefinitionId: String?, dateFilter: DashboardDateFilter) {
        selectedIncidentProcessDefinitionId = processDefinitionId
        selectedIncidentDateFilter = dateFilter
        activeProcessTabId = null
        destination = AppDestination.Incidents
    }

    fun openProcessDefinition(
        processDefinitionId: String,
        dateFilter: DashboardDateFilter = DashboardDateFilter(),
    ) {
        selectedProcessDefinitionId = processDefinitionId
        selectedDashboardDateFilter = dateFilter
        selectedProcessInstanceId = null
        activeProcessTabId = null
        destination = AppDestination.ProcessDefinition
    }

    fun openProcessInstance(processInstanceId: String) {
        val connection = selectedConnection ?: return
        processInstanceReturnDestination = destination
        val tabId = processTabId(connection.id, processInstanceId)
        if (openProcessTabs.none { it.id == tabId }) {
            if (openProcessTabs.size >= MAX_OPEN_PROCESS_TABS) openProcessTabs.removeAt(0)
            openProcessTabs += OpenProcessTab(
                id = tabId,
                connectionId = connection.id,
                connectionName = connection.name,
                processInstanceId = processInstanceId,
                returnDestination = processInstanceReturnDestination,
            )
        }
        activeProcessTabId = tabId
        selectedProcessInstanceId = processInstanceId
        destination = AppDestination.ProcessInstance
    }

    fun activateProcessTab(tabId: String) {
        val tab = openProcessTabs.firstOrNull { it.id == tabId } ?: return
        if (connections.none { it.id == tab.connectionId }) {
            openProcessTabs.remove(tab)
            return
        }
        selectedConnectionId = tab.connectionId
        selectedProcessDefinitionId = null
        selectedProcessInstanceId = tab.processInstanceId
        processInstanceReturnDestination = tab.returnDestination
        activeProcessTabId = tab.id
        destination = AppDestination.ProcessInstance
    }

    fun closeProcessTab(tabId: String) {
        val index = openProcessTabs.indexOfFirst { it.id == tabId }
        if (index < 0) return
        val wasActive = activeProcessTabId == tabId
        openProcessTabs.removeAt(index)
        if (!wasActive) return
        val replacement = openProcessTabs.getOrNull(index.coerceAtMost(openProcessTabs.lastIndex))
        if (replacement != null) {
            activateProcessTab(replacement.id)
        } else {
            activeProcessTabId = null
            selectedProcessInstanceId = null
            destination = AppDestination.Overview
        }
    }

    fun updateOpenProcessTab(instance: ProcessInstanceSummary) {
        val connectionId = selectedConnectionId ?: return
        val tabId = processTabId(connectionId, instance.id)
        val index = openProcessTabs.indexOfFirst { it.id == tabId }
        if (index < 0) return
        openProcessTabs[index] = openProcessTabs[index].copy(
            businessKey = instance.businessKey,
            processDefinitionKey = instance.definitionKey,
        )
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
        activeProcessTabId = null
        destination = AppDestination.Overview
    }

    fun backFromProcessInstance() {
        destination = if (
            processInstanceReturnDestination == AppDestination.ProcessDefinition && selectedProcessDefinitionId == null
        ) AppDestination.Overview else processInstanceReturnDestination
        selectedProcessInstanceId = null
        activeProcessTabId = null
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
        openProcessTabs.removeAll { it.connectionId == connectionId }
        if (selectedConnectionId == connectionId) {
            selectedConnectionId = connections.firstOrNull()?.id
        }
    }

    fun updateProcessSearch(value: String) {
        processSearch = value
    }

    private fun processTabId(connectionId: String, processInstanceId: String): String =
        "$connectionId::$processInstanceId"

    private companion object {
        const val MAX_OPEN_PROCESS_TABS = 12
    }
}
