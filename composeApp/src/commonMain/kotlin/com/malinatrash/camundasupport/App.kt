package com.malinatrash.camundasupport

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.data.ConnectionRepository
import com.malinatrash.camundasupport.data.ConnectionTester
import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.ExternalNavigator
import com.malinatrash.camundasupport.data.APP_VERSION
import com.malinatrash.camundasupport.data.AppUpdate
import com.malinatrash.camundasupport.data.AppUpdateService
import com.malinatrash.camundasupport.data.NoOpAppUpdateService
import com.malinatrash.camundasupport.data.UpdateCheckResult
import com.malinatrash.camundasupport.data.UpdateDownloadProgress
import com.malinatrash.camundasupport.data.UpdateInstallResult
import com.malinatrash.camundasupport.data.VariableKeyRepository
import com.malinatrash.camundasupport.model.AppDestination
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.cockpitDashboardUrl
import com.malinatrash.camundasupport.model.processDefinitionCockpitUrl
import com.malinatrash.camundasupport.model.processInstanceCockpitUrl
import com.malinatrash.camundasupport.state.AppStore
import com.malinatrash.camundasupport.state.DashboardPollingStore
import com.malinatrash.camundasupport.ui.components.AppSidebar
import com.malinatrash.camundasupport.ui.components.ConnectionDialog
import com.malinatrash.camundasupport.ui.components.EnvironmentBadge
import com.malinatrash.camundasupport.ui.components.UpdateBanner
import com.malinatrash.camundasupport.ui.screens.ConnectionsScreen
import com.malinatrash.camundasupport.ui.screens.IncidentsScreen
import com.malinatrash.camundasupport.ui.screens.OverviewScreen
import com.malinatrash.camundasupport.ui.screens.ProcessDefinitionDetailScreen
import com.malinatrash.camundasupport.ui.screens.ProcessInstanceDetailScreen
import com.malinatrash.camundasupport.ui.screens.ProcessesScreen
import com.malinatrash.camundasupport.ui.screens.SettingsScreen
import com.malinatrash.camundasupport.ui.theme.AppBackground
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.SupportTheme
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

@Composable
fun CamundaSupportApp(
    connectionRepository: ConnectionRepository,
    connectionTester: ConnectionTester,
    camundaApi: CamundaApi,
    variableKeyRepository: VariableKeyRepository,
    externalNavigator: ExternalNavigator,
    updateService: AppUpdateService = NoOpAppUpdateService,
) {
    val store = remember(connectionRepository) { AppStore(connectionRepository) }
    val dashboardStore = remember(camundaApi) { DashboardPollingStore(camundaApi) }
    val coroutineScope = rememberCoroutineScope()
    var availableUpdate by remember { mutableStateOf<AppUpdate?>(null) }
    var dismissedUpdateTag by remember { mutableStateOf<String?>(null) }
    var updateDownloading by remember { mutableStateOf(false) }
    var updateDownloadProgress by remember { mutableStateOf<UpdateDownloadProgress?>(null) }
    var updateError by remember { mutableStateOf<String?>(null) }
    var openedInstaller by remember { mutableStateOf<String?>(null) }
    SideEffect { dashboardStore.selectConnection(store.selectedConnection) }
    val dashboardQuery = dashboardStore.activeQuery

    LaunchedEffect(dashboardQuery) {
        val query = dashboardQuery ?: return@LaunchedEffect
        while (currentCoroutineContext().isActive) {
            dashboardStore.updateCountdown(0)
            dashboardStore.refresh(query)
            for (seconds in DashboardPollingStore.POLLING_INTERVAL.inWholeSeconds.toInt() downTo 1) {
                dashboardStore.updateCountdown(seconds)
                delay(1_000)
            }
        }
    }

    LaunchedEffect(updateService) {
        while (currentCoroutineContext().isActive) {
            when (val result = updateService.checkForUpdate(APP_VERSION)) {
                is UpdateCheckResult.Available -> availableUpdate = result.update
                is UpdateCheckResult.UpToDate -> availableUpdate = null
                is UpdateCheckResult.Failure -> Unit
            }
            delay(UPDATE_CHECK_INTERVAL_MILLIS)
        }
    }

    LaunchedEffect(availableUpdate?.tag) {
        updateDownloadProgress = null
        updateError = null
        openedInstaller = null
    }

    SupportTheme {
        Row(Modifier.fillMaxSize().background(AppBackground)) {
            AppSidebar(
                connections = store.connections,
                selectedConnectionId = store.selectedConnectionId,
                destination = store.destination,
                onSelectConnection = store::selectConnection,
                onNavigate = store::navigate,
                onAddConnection = store::openConnectionDialog,
            )

            Box(Modifier.width(1.dp).fillMaxHeight().background(Border))

            Column(Modifier.fillMaxSize()) {
                ContextBar(
                    connection = store.selectedConnection,
                    onAddConnection = store::openConnectionDialog,
                    onOpenCockpit = { url -> externalNavigator.open(url) },
                )
                availableUpdate
                    ?.takeIf { it.tag != dismissedUpdateTag }
                    ?.let { update ->
                        UpdateBanner(
                            update = update,
                            downloading = updateDownloading,
                            progress = updateDownloadProgress,
                            error = updateError,
                            openedInstaller = openedInstaller,
                            onOpenRelease = { externalNavigator.open(update.releaseUrl) },
                            onInstall = {
                                if (update.asset == null) {
                                    externalNavigator.open(update.releaseUrl)
                                } else {
                                    coroutineScope.launch {
                                        updateDownloading = true
                                        updateError = null
                                        openedInstaller = null
                                        updateDownloadProgress = null
                                        when (
                                            val result = updateService.downloadAndOpen(update) { progress ->
                                                coroutineScope.launch { updateDownloadProgress = progress }
                                            }
                                        ) {
                                            is UpdateInstallResult.InstallerOpened -> openedInstaller = result.fileName
                                            is UpdateInstallResult.Failure -> updateError = result.message
                                        }
                                        updateDownloading = false
                                    }
                                }
                            },
                            onDismiss = { dismissedUpdateTag = update.tag },
                        )
                    }
                HorizontalDivider(color = Border)
                Box(
                    modifier = Modifier.fillMaxSize().padding(horizontal = 20.dp, vertical = 16.dp),
                ) {
                    when (store.destination) {
                        AppDestination.Overview -> OverviewScreen(
                            connection = store.selectedConnection,
                            dashboardStore = dashboardStore,
                            onAddConnection = store::openConnectionDialog,
                            onOpenDefinition = store::openProcessDefinition,
                            onOpenIncidents = store::openIncidents,
                            onOpenInstance = store::openProcessInstance,
                        )

                        AppDestination.Processes -> ProcessesScreen(
                            connection = store.selectedConnection,
                            camundaApi = camundaApi,
                            variableKeyRepository = variableKeyRepository,
                            onAddConnection = store::openConnectionDialog,
                            onOpenInstance = store::openProcessInstance,
                        )

                        AppDestination.Incidents -> IncidentsScreen(
                            connection = store.selectedConnection,
                            camundaApi = camundaApi,
                            processDefinitionId = store.selectedIncidentProcessDefinitionId,
                            dateFilter = store.selectedIncidentDateFilter,
                            onAddConnection = store::openConnectionDialog,
                            onOpenInstance = store::openProcessInstance,
                        )

                        AppDestination.Connections -> ConnectionsScreen(
                            connections = store.connections,
                            selectedConnectionId = store.selectedConnectionId,
                            onSelectConnection = store::selectConnection,
                            onAddConnection = store::openConnectionDialog,
                            onDeleteConnection = store::deleteConnection,
                        )

                        AppDestination.Settings -> SettingsScreen()

                        AppDestination.ProcessDefinition -> {
                            val connection = store.selectedConnection
                            val definitionId = store.selectedProcessDefinitionId
                            if (connection != null && definitionId != null) {
                                ProcessDefinitionDetailScreen(
                                    connection = connection,
                                    processDefinitionId = definitionId,
                                    dateFilter = store.selectedDashboardDateFilter,
                                    camundaApi = camundaApi,
                                    onBack = store::backFromProcessDefinition,
                                    onOpenInstance = store::openProcessInstance,
                                    onOpenBrowser = {
                                        connection.processDefinitionCockpitUrl(definitionId)
                                            ?.let(externalNavigator::open)
                                    },
                                )
                            }
                        }

                        AppDestination.ProcessInstance -> {
                            val connection = store.selectedConnection
                            val instanceId = store.selectedProcessInstanceId
                            if (connection != null && instanceId != null) {
                                ProcessInstanceDetailScreen(
                                    connection = connection,
                                    processInstanceId = instanceId,
                                    camundaApi = camundaApi,
                                    onBack = store::backFromProcessInstance,
                                    onOpenBrowser = {
                                        connection.processInstanceCockpitUrl(instanceId)
                                            ?.let(externalNavigator::open)
                                    },
                                )
                            }
                        }
                    }
                }
            }
        }

        if (store.isConnectionDialogOpen) {
            ConnectionDialog(
                connectionTester = connectionTester,
                onDismiss = store::closeConnectionDialog,
                onSave = store::addConnection,
            )
        }
    }
}

private const val UPDATE_CHECK_INTERVAL_MILLIS = 6L * 60 * 60 * 1_000

@Composable
private fun ContextBar(
    connection: CamundaConnection?,
    onAddConnection: () -> Unit,
    onOpenCockpit: (String) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().height(58.dp).padding(horizontal = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (connection == null) {
            Column(Modifier.weight(1f)) {
                Text("Подключение не выбрано", fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                Text("Добавьте окружение Camunda REST, чтобы продолжить", color = TextSecondary, fontSize = 11.sp)
            }
            Button(onClick = onAddConnection) { Text("Добавить подключение") }
            return@Row
        }

        Column(Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(connection.name, fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                Spacer(Modifier.width(9.dp))
                EnvironmentBadge(connection.environment)
            }
            Text(
                text = connection.restUrl,
                color = TextSecondary,
                fontSize = 10.sp,
                fontFamily = FontFamily.Monospace,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }

        Text(
            connection.engineVersion?.let { "CAMUNDA $it" } ?: "ПОДКЛЮЧЕНО",
            color = TextSecondary,
            fontSize = 9.sp,
            fontWeight = FontWeight.Bold,
            letterSpacing = 0.6.sp,
        )
        Spacer(Modifier.width(16.dp))
        OutlinedButton(
            onClick = { connection.cockpitDashboardUrl()?.let(onOpenCockpit) },
            enabled = connection.cockpitDashboardUrl() != null,
        ) {
            Text("Cockpit ↗")
        }
    }
}
