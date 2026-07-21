package com.malinatrash.camundasupport.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.DisableSelection
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.DialogWindow
import androidx.compose.ui.window.rememberDialogState
import kotlinx.coroutines.launch
import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.model.ActiveActivityInstance
import com.malinatrash.camundasupport.model.BpmnNode
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.model.ProcessExternalTask
import com.malinatrash.camundasupport.model.ProcessIncident
import com.malinatrash.camundasupport.model.ProcessInstanceDetails
import com.malinatrash.camundasupport.model.ProcessJob
import com.malinatrash.camundasupport.model.ProcessVariable
import com.malinatrash.camundasupport.model.ProcessVariableUpdate
import com.malinatrash.camundasupport.model.TeleportRequest
import com.malinatrash.camundasupport.ui.components.BpmnViewer
import com.malinatrash.camundasupport.ui.components.EmptyPanel
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.Healthy
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.PrimaryMuted
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary
import com.malinatrash.camundasupport.ui.theme.Warning

private enum class InstanceTab(val label: String) {
    Diagram("Схема"),
    Diagnosis("Диагностика"),
    Variables("Переменные"),
    Metadata("Метаданные"),
}

@Composable
fun ProcessInstanceDetailScreen(
    connection: CamundaConnection,
    processInstanceId: String,
    camundaApi: CamundaApi,
    onBack: () -> Unit,
    onOpenBrowser: () -> Unit,
) {
    var details by remember { mutableStateOf<ProcessInstanceDetails?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var operationMessage by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }
    var operationBusy by remember { mutableStateOf(false) }
    var refreshToken by remember { mutableIntStateOf(0) }
    var teleportOpen by remember { mutableStateOf(false) }
    var teleportTargetId by remember(processInstanceId) { mutableStateOf<String?>(null) }
    var selectedNodeId by remember(processInstanceId) { mutableStateOf<String?>(null) }
    var variableToEdit by remember { mutableStateOf<ProcessVariable?>(null) }
    var variableEditError by remember { mutableStateOf<String?>(null) }
    var pendingAction by remember { mutableStateOf<PendingAction?>(null) }
    var selectedTab by remember { mutableStateOf(InstanceTab.Diagram) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(connection.id, processInstanceId, refreshToken) {
        loading = true
        error = null
        when (val result = camundaApi.loadProcessInstanceDetails(connection, processInstanceId)) {
            is CamundaApiResult.Success -> details = result.value
            is CamundaApiResult.Failure -> error = result.message
        }
        loading = false
    }

    fun handleOperationResult(result: CamundaApiResult<Unit>, successMessage: String) {
        operationBusy = false
        when (result) {
            is CamundaApiResult.Success -> {
                operationMessage = successMessage
                refreshToken += 1
            }
            is CamundaApiResult.Failure -> error = result.message
        }
    }

    fun execute(action: PendingAction) {
        operationBusy = true
        error = null
        pendingAction = null
        scope.launch {
            when (action) {
                is PendingAction.JobRetry -> handleOperationResult(
                    camundaApi.setJobRetries(connection, action.jobId, 3),
                    "Для задания движка установлено 3 попытки.",
                )
                is PendingAction.ExternalRetry -> handleOperationResult(
                    camundaApi.setExternalTaskRetries(connection, action.taskId, 3),
                    "Для внешней задачи установлено 3 попытки.",
                )
                is PendingAction.ExternalUnlock -> handleOperationResult(
                    camundaApi.unlockExternalTask(connection, action.taskId),
                    "Внешняя задача разблокирована.",
                )
                is PendingAction.Suspension -> handleOperationResult(
                    camundaApi.setProcessInstanceSuspended(connection, processInstanceId, action.suspended),
                    if (action.suspended) "Заявка приостановлена." else "Заявка активирована.",
                )
            }
        }
    }

    Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        CompactInstanceHeader(
            processInstanceId = processInstanceId,
            details = details,
            loading = loading,
            operationBusy = operationBusy,
            onBack = onBack,
            onRefresh = { refreshToken += 1 },
            onOpenBrowser = onOpenBrowser,
            onTeleport = {
                teleportTargetId = null
                teleportOpen = true
            },
            onToggleSuspension = { snapshot ->
                pendingAction = PendingAction.Suspension(!snapshot.instance.suspended)
            },
        )

        error?.let { InlineMessage(it, Danger) }
        operationMessage?.let { InlineMessage(it, Healthy) }

        if (details == null && loading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
            return@Column
        }
        val snapshot = details
        if (snapshot == null) {
            EmptyPanel(
                title = "Не удалось загрузить заявку",
                description = error ?: "Camunda не вернула данные экземпляра процесса.",
                actionLabel = "Повторить",
                onAction = { refreshToken += 1 },
            )
            return@Column
        }

        InstanceTabs(selectedTab, onSelect = { selectedTab = it })
        Box(Modifier.weight(1f).fillMaxWidth()) {
            when (selectedTab) {
                InstanceTab.Diagram -> DiagramTab(
                    snapshot = snapshot,
                    selectedNodeId = selectedNodeId,
                    disabled = operationBusy,
                    onNodeSelected = { selectedNodeId = it },
                    onTeleport = { node ->
                        teleportTargetId = node.id
                        teleportOpen = true
                    },
                    onJobRetry = { pendingAction = PendingAction.JobRetry(it) },
                    onExternalRetry = { pendingAction = PendingAction.ExternalRetry(it) },
                    onExternalUnlock = { pendingAction = PendingAction.ExternalUnlock(it) },
                )
                InstanceTab.Diagnosis -> DiagnosisTab(
                    snapshot = snapshot,
                    disabled = operationBusy,
                    onJobRetry = { pendingAction = PendingAction.JobRetry(it) },
                    onExternalRetry = { pendingAction = PendingAction.ExternalRetry(it) },
                    onExternalUnlock = { pendingAction = PendingAction.ExternalUnlock(it) },
                )
                InstanceTab.Variables -> VariablesTab(
                    variables = snapshot.variables,
                    disabled = operationBusy,
                    onEdit = {
                        variableEditError = null
                        variableToEdit = it
                    },
                )
                InstanceTab.Metadata -> MetadataTab(snapshot)
            }
        }
    }

    pendingAction?.let { action ->
        DisableSelection {
            ConfirmOperationDialog(
                action = action,
                production = connection.environment == Environment.Production,
                onDismiss = { pendingAction = null },
                onConfirm = { execute(action) },
            )
        }
    }

    if (teleportOpen && details != null) {
        DisableSelection {
            TeleportDialog(
                details = details!!,
                initialTargetId = teleportTargetId,
                production = connection.environment == Environment.Production,
                busy = operationBusy,
                onDismiss = { teleportOpen = false },
                onExecute = { request ->
                    operationBusy = true
                    scope.launch {
                        val result = camundaApi.teleportProcessInstance(connection, processInstanceId, request)
                        if (result is CamundaApiResult.Success) teleportOpen = false
                        handleOperationResult(result, "Телепорт выполнен, состояние заявки обновлено.")
                    }
                },
            )
        }
    }

    variableToEdit?.let { variable ->
        DisableSelection {
            VariableEditDialog(
                variable = variable,
                production = connection.environment == Environment.Production,
                busy = operationBusy,
                operationError = variableEditError,
                onDismiss = {
                    if (!operationBusy) variableToEdit = null
                },
                onSave = { newValue ->
                    operationBusy = true
                    variableEditError = null
                    scope.launch {
                        val result = camundaApi.updateProcessVariable(
                            connection = connection,
                            processInstanceId = processInstanceId,
                            update = ProcessVariableUpdate(
                                name = variable.name,
                                type = variable.type,
                                value = newValue,
                                valueInfo = variable.valueInfo,
                            ),
                        )
                        when (result) {
                            is CamundaApiResult.Success -> {
                                variableToEdit = null
                                handleOperationResult(result, "Переменная ${variable.name} обновлена.")
                            }
                            is CamundaApiResult.Failure -> {
                                operationBusy = false
                                variableEditError = result.message
                            }
                        }
                    }
                },
            )
        }
    }
}

@Composable
private fun CompactInstanceHeader(
    processInstanceId: String,
    details: ProcessInstanceDetails?,
    loading: Boolean,
    operationBusy: Boolean,
    onBack: () -> Unit,
    onRefresh: () -> Unit,
    onOpenBrowser: () -> Unit,
    onTeleport: () -> Unit,
    onToggleSuspension: (ProcessInstanceDetails) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            OutlinedButton(onClick = onBack) { Text("← Назад") }
            Spacer(Modifier.width(10.dp))
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Заявка", fontSize = 20.sp, fontWeight = FontWeight.SemiBold)
                    details?.let { InstanceStatus(it.instance.suspended) }
                }
                Text(processInstanceId, color = TextSecondary, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
            }
            OutlinedButton(onClick = onRefresh, enabled = !loading && !operationBusy) {
                Text(if (loading) "Обновляем…" else "Обновить")
            }
            Spacer(Modifier.width(6.dp))
            OutlinedButton(onClick = onOpenBrowser) { Text("В браузере ↗") }
            if (details != null) {
                Spacer(Modifier.width(6.dp))
                OutlinedButton(onClick = { onToggleSuspension(details) }, enabled = !operationBusy) {
                    Text(if (details.instance.suspended) "Активировать" else "Приостановить")
                }
                Spacer(Modifier.width(6.dp))
                Button(onClick = onTeleport, enabled = !operationBusy && details.activeActivities.isNotEmpty()) {
                    Text("Телепорт")
                }
            }
        }
        details?.let { snapshot ->
            Row(
                Modifier.fillMaxWidth().background(Surface, RoundedCornerShape(9.dp)).padding(horizontal = 12.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(20.dp),
            ) {
                HeaderFact("Бизнес-ключ", snapshot.instance.businessKey ?: "—")
                HeaderFact("Процесс", snapshot.instance.definitionKey)
                HeaderFact("Активных элементов", snapshot.activeActivities.size.toString())
                HeaderFact("Инцидентов", snapshot.incidents.size.toString(), if (snapshot.incidents.isEmpty()) Healthy else Danger)
            }
        }
    }
}

@Composable
private fun HeaderFact(label: String, value: String, color: Color = TextPrimary) {
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("$label:", color = TextSecondary, fontSize = 11.sp)
        Text(value, color = color, fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun InstanceTabs(selected: InstanceTab, onSelect: (InstanceTab) -> Unit) {
    Row(
        Modifier.fillMaxWidth().background(PrimaryMuted.copy(alpha = 0.45f), RoundedCornerShape(9.dp)).padding(4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        InstanceTab.entries.forEach { tab ->
            val active = tab == selected
            Box(
                Modifier
                    .background(if (active) Surface else Color.Transparent, RoundedCornerShape(7.dp))
                    .clickable { onSelect(tab) }
                    .padding(horizontal = 16.dp, vertical = 8.dp),
            ) {
                Text(tab.label, color = if (active) Primary else TextSecondary, fontWeight = if (active) FontWeight.SemiBold else FontWeight.Normal, fontSize = 12.sp)
            }
        }
    }
}

@Composable
private fun DiagramTab(
    snapshot: ProcessInstanceDetails,
    selectedNodeId: String?,
    disabled: Boolean,
    onNodeSelected: (String) -> Unit,
    onTeleport: (BpmnNode) -> Unit,
    onJobRetry: (String) -> Unit,
    onExternalRetry: (String) -> Unit,
    onExternalUnlock: (String) -> Unit,
) {
    val selectedNode = snapshot.diagram.nodes.firstOrNull { it.id == selectedNodeId }
    val completedCounts = snapshot.activityHistory
        .filter { it.completedCount > 0 }
        .associate { it.activityId to it.completedCount }
    Row(Modifier.fillMaxSize(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Card(
            Modifier.weight(1f).fillMaxHeight(),
            colors = CardDefaults.cardColors(containerColor = Surface),
            border = BorderStroke(1.dp, Border),
            shape = RoundedCornerShape(10.dp),
        ) {
            Column(Modifier.fillMaxSize()) {
                DiagramProgressHeader(snapshot)
                HorizontalDivider(color = Border)
                if (snapshot.bpmnXml.isBlank()) {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text("Camunda не вернула BPMN XML", color = TextSecondary)
                    }
                } else {
                    DisableSelection {
                        BpmnViewer(
                            xml = snapshot.bpmnXml,
                            activeActivityIds = snapshot.activeActivities.mapTo(mutableSetOf(), ActiveActivityInstance::activityId),
                            incidentActivityIds = snapshot.incidents.mapNotNullTo(mutableSetOf()) { it.failedActivityId ?: it.activityId },
                            completedActivityCounts = completedCounts,
                            clickableActivityIds = snapshot.diagram.teleportTargets.mapTo(mutableSetOf(), BpmnNode::id),
                            onActivityClick = onNodeSelected,
                            modifier = Modifier.fillMaxSize(),
                        )
                    }
                }
            }
        }
        Column(Modifier.width(320.dp).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            NodeActionsCard(
                node = selectedNode,
                snapshot = snapshot,
                disabled = disabled,
                onTeleport = onTeleport,
                onJobRetry = onJobRetry,
                onExternalRetry = onExternalRetry,
                onExternalUnlock = onExternalUnlock,
            )
            CompactPanel("ТЕКУЩЕЕ СОСТОЯНИЕ", "${snapshot.activeActivities.size}") {
                LazyColumn(verticalArrangement = Arrangement.spacedBy(7.dp)) {
                    items(snapshot.activeActivities, key = ActiveActivityInstance::id) { ActivityRow(it) }
                }
            }
            CompactPanel("ИНЦИДЕНТЫ", snapshot.incidents.size.toString()) {
                if (snapshot.incidents.isEmpty()) {
                    Text("Открытых инцидентов нет", color = Healthy, fontSize = 12.sp)
                } else {
                    LazyColumn(verticalArrangement = Arrangement.spacedBy(7.dp)) {
                        items(snapshot.incidents, key = ProcessIncident::id) { IncidentRow(it) }
                    }
                }
            }
        }
    }
}

@Composable
private fun DiagramProgressHeader(snapshot: ProcessInstanceDetails) {
    val diagramActivityIds = snapshot.diagram.teleportTargets.mapTo(mutableSetOf(), BpmnNode::id)
    val completed = snapshot.activityHistory.filter { it.activityId in diagramActivityIds && it.completedCount > 0 }
    val completedExecutions = completed.sumOf { it.completedCount }
    val repeats = completed.sumOf { (it.completedCount - 1).coerceAtLeast(0) }
    val totalCubes = snapshot.diagram.teleportTargets.size

    Column(Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 9.dp), verticalArrangement = Arrangement.spacedBy(5.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("BPMN-ПУТЬ ЗАЯВКИ", color = TextPrimary, fontSize = 12.sp, fontWeight = FontWeight.Bold)
            Spacer(Modifier.weight(1f))
            HeaderFact("Пройдено", "${completed.size} из $totalCubes", Healthy)
            Spacer(Modifier.width(14.dp))
            HeaderFact("Выполнений", completedExecutions.toString())
            if (repeats > 0) {
                Spacer(Modifier.width(14.dp))
                HeaderFact("Повторов", repeats.toString(), Warning)
            }
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            DiagramLegend("● пройден", Healthy)
            Spacer(Modifier.width(12.dp))
            DiagramLegend("● активен", Primary)
            Spacer(Modifier.width(12.dp))
            DiagramLegend("● инцидент", Danger)
            Text(
                "Нажмите кубик для действий · Ctrl/⌘ + колесо — масштаб",
                modifier = Modifier.weight(1f).padding(start = 12.dp),
                color = TextSecondary,
                fontSize = 10.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun DiagramLegend(text: String, color: Color) {
    Text(text, color = color, fontSize = 10.sp, fontWeight = FontWeight.SemiBold)
}

@Composable
private fun NodeActionsCard(
    node: BpmnNode?,
    snapshot: ProcessInstanceDetails,
    disabled: Boolean,
    onTeleport: (BpmnNode) -> Unit,
    onJobRetry: (String) -> Unit,
    onExternalRetry: (String) -> Unit,
    onExternalUnlock: (String) -> Unit,
) {
    Card(
        Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = if (node == null) PrimaryMuted.copy(alpha = 0.35f) else Surface),
        border = BorderStroke(1.dp, if (node == null) Primary.copy(alpha = 0.25f) else Primary.copy(alpha = 0.55f)),
        shape = RoundedCornerShape(10.dp),
    ) {
        if (node == null) {
            Column(Modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("ДЕЙСТВИЯ С КУБИКОМ", color = Primary, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                Text("Нажмите на кубик в BPMN-схеме", fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                Text("Покажем доступные действия именно для выбранного элемента.", color = TextSecondary, fontSize = 10.sp)
            }
            return@Card
        }

        val active = snapshot.activeActivities.any { it.activityId == node.id }
        val history = snapshot.activityHistory.firstOrNull { it.activityId == node.id }
        val incidents = snapshot.incidents.filter { (it.failedActivityId ?: it.activityId) == node.id }
        val jobs = snapshot.jobs.filter { it.failedActivityId == node.id }
        val externalTasks = snapshot.externalTasks.filter { it.activityId == node.id }
        val canTeleport = snapshot.diagram.teleportTargets.any { it.id == node.id }

        Column(Modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("ВЫБРАННЫЙ КУБИК", color = Primary, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    Text(node.name ?: node.id, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                history?.completedCount?.takeIf { it > 0 }?.let { NodeStateBadge("ПРОЙДЕН · ×$it", Healthy) }
                if (active) NodeStateBadge("АКТИВЕН", Primary)
                if (incidents.isNotEmpty()) NodeStateBadge("ОШИБКА · ${incidents.size}", Danger)
                history?.canceledCount?.takeIf { it > 0 }?.let { NodeStateBadge("ОТМЕНЁН · $it", Warning) }
            }
            Text(
                listOfNotNull(node.type, node.topic?.let { "topic: $it" }, node.id).joinToString(" · "),
                color = TextSecondary,
                fontFamily = FontFamily.Monospace,
                fontSize = 9.sp,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            HorizontalDivider(color = Border)
            Button(
                onClick = { onTeleport(node) },
                enabled = !disabled && canTeleport && snapshot.activeActivities.isNotEmpty(),
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Телепортировать сюда") }

            jobs.firstOrNull()?.let { job ->
                OutlinedButton(onClick = { onJobRetry(job.id) }, enabled = !disabled, modifier = Modifier.fillMaxWidth()) {
                    Text("Дать 3 попытки заданию")
                }
            }
            externalTasks.firstOrNull()?.let { task ->
                OutlinedButton(onClick = { onExternalRetry(task.id) }, enabled = !disabled, modifier = Modifier.fillMaxWidth()) {
                    Text("Дать 3 попытки внешней задаче")
                }
                OutlinedButton(
                    onClick = { onExternalUnlock(task.id) },
                    enabled = !disabled && task.workerId != null,
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Разблокировать внешнюю задачу") }
            }
            if (!canTeleport && jobs.isEmpty() && externalTasks.isEmpty()) {
                Text("Для этого элемента нет безопасных ручных операций.", color = TextSecondary, fontSize = 10.sp)
            }
            if (jobs.size > 1 || externalTasks.size > 1) {
                Text("Другие задания этого кубика доступны во вкладке «Диагностика».", color = TextSecondary, fontSize = 9.sp)
            }
        }
    }
}

@Composable
private fun NodeStateBadge(text: String, color: Color) {
    Text(
        text,
        color = color,
        fontSize = 8.sp,
        fontWeight = FontWeight.Bold,
        modifier = Modifier.padding(start = 5.dp).background(color.copy(alpha = 0.10f), RoundedCornerShape(5.dp)).padding(horizontal = 5.dp, vertical = 3.dp),
    )
}

@Composable
private fun DiagnosisTab(
    snapshot: ProcessInstanceDetails,
    disabled: Boolean,
    onJobRetry: (String) -> Unit,
    onExternalRetry: (String) -> Unit,
    onExternalUnlock: (String) -> Unit,
) {
    LazyColumn(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        item {
            DetailSection("ИНЦИДЕНТЫ", "${snapshot.incidents.size}") {
                if (snapshot.incidents.isEmpty()) Text("Открытых инцидентов нет", color = Healthy)
                snapshot.incidents.forEach { IncidentRow(it) }
            }
        }
        item {
            DetailSection("ЗАДАНИЯ ДВИЖКА", "${snapshot.jobs.size}") {
                if (snapshot.jobs.isEmpty()) Text("Задания движка не найдены", color = TextSecondary)
                snapshot.jobs.forEach { job -> JobRow(job, disabled) { onJobRetry(job.id) } }
            }
        }
        item {
            DetailSection("ВНЕШНИЕ ЗАДАЧИ", "${snapshot.externalTasks.size}") {
                if (snapshot.externalTasks.isEmpty()) Text("Внешние задачи не найдены", color = TextSecondary)
                snapshot.externalTasks.forEach { task ->
                    ExternalTaskRow(
                        task = task,
                        disabled = disabled,
                        onRetry = { onExternalRetry(task.id) },
                        onUnlock = { onExternalUnlock(task.id) },
                    )
                }
            }
        }
    }
}

@Composable
private fun VariablesTab(
    variables: List<ProcessVariable>,
    disabled: Boolean,
    onEdit: (ProcessVariable) -> Unit,
) {
    var query by remember { mutableStateOf("") }
    val filtered = variables.filter { variable ->
        query.isBlank() || variable.name.contains(query, true) || variable.value.contains(query, true) || variable.type.contains(query, true)
    }
    Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Переменные Camunda", fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
            Text("${filtered.size} из ${variables.size}", color = TextSecondary, fontSize = 11.sp)
            Spacer(Modifier.width(10.dp))
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = { Text("Поиск по ключу, типу или значению") },
                singleLine = true,
                modifier = Modifier.width(360.dp),
            )
        }
        LazyColumn(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            items(filtered, key = ProcessVariable::name) { variable ->
                VariableRow(
                    variable = variable,
                    editEnabled = !disabled && variable.isEditable,
                    onEdit = { onEdit(variable) },
                )
            }
        }
    }
}

@Composable
private fun MetadataTab(snapshot: ProcessInstanceDetails) {
    DetailSection("МЕТАДАННЫЕ ЗАЯВКИ", "${snapshot.metadata.size} полей") {
        snapshot.metadata.forEach { field ->
            Row(Modifier.fillMaxWidth().padding(vertical = 3.dp)) {
                Text(field.label, modifier = Modifier.width(250.dp), color = TextSecondary, fontSize = 11.sp)
                Text(field.value, modifier = Modifier.weight(1f), fontFamily = FontFamily.Monospace, fontSize = 11.sp)
            }
        }
    }
}

@Composable
private fun PanelHeader(title: String, subtitle: String) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 9.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(title, fontSize = 11.sp, fontWeight = FontWeight.Bold)
        Spacer(Modifier.weight(1f))
        Text(subtitle, color = TextSecondary, fontSize = 10.sp)
    }
}

@Composable
private fun ColumnScope.CompactPanel(title: String, count: String, content: @Composable ColumnScope.() -> Unit) {
    Card(
        Modifier.weight(1f).fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(10.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row {
                Text(title, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.weight(1f))
                Text(count, color = TextSecondary, fontFamily = FontFamily.Monospace, fontSize = 10.sp)
            }
            HorizontalDivider(color = Border)
            content()
        }
    }
}

@Composable
private fun DetailSection(title: String, subtitle: String, content: @Composable ColumnScope.() -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(10.dp),
    ) {
        Column(Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(9.dp)) {
            Row {
                Text(title, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.weight(1f))
                Text(subtitle, color = TextSecondary, fontSize = 10.sp)
            }
            HorizontalDivider(color = Border)
            content()
        }
    }
}

@Composable
private fun InstanceStatus(suspended: Boolean) {
    val color = if (suspended) Warning else Healthy
    Text(
        if (suspended) "ПРИОСТАНОВЛЕНА" else "ЗАПУЩЕНА",
        color = color,
        fontSize = 9.sp,
        fontWeight = FontWeight.Bold,
        modifier = Modifier.background(color.copy(alpha = 0.12f), RoundedCornerShape(6.dp)).padding(horizontal = 7.dp, vertical = 4.dp),
    )
}

@Composable
private fun ActivityRow(activity: ActiveActivityInstance) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.width(7.dp).height(7.dp).background(if (activity.incidentIds.isEmpty()) Primary else Danger, RoundedCornerShape(4.dp)))
        Spacer(Modifier.width(8.dp))
        Column(Modifier.weight(1f)) {
            Text(activity.activityName ?: activity.activityId, fontWeight = FontWeight.Medium, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(activity.activityId, color = TextSecondary, fontSize = 9.sp, fontFamily = FontFamily.Monospace, maxLines = 1)
        }
    }
}

@Composable
private fun IncidentRow(incident: ProcessIncident) {
    Column(
        Modifier.fillMaxWidth().background(Danger.copy(alpha = 0.06f), RoundedCornerShape(7.dp)).padding(9.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Text(incident.failedActivityId ?: incident.activityId ?: incident.type, color = Danger, fontWeight = FontWeight.SemiBold, fontSize = 11.sp)
        Text(incident.message ?: "Сообщение отсутствует", fontSize = 11.sp, maxLines = 3, overflow = TextOverflow.Ellipsis)
    }
}

@Composable
private fun JobRow(job: ProcessJob, disabled: Boolean, onRetry: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text("Задание · осталось попыток: ${job.retries}", fontWeight = FontWeight.Medium, fontSize = 12.sp)
            Text(job.exceptionMessage ?: "Ошибка отсутствует", color = if (job.exceptionMessage != null) Danger else TextSecondary, fontSize = 11.sp, maxLines = 2)
            Text(job.failedActivityId ?: job.id, color = TextSecondary, fontSize = 9.sp, fontFamily = FontFamily.Monospace)
        }
        Button(onClick = onRetry, enabled = !disabled) { Text("Дать 3 попытки") }
    }
}

@Composable
private fun ExternalTaskRow(task: ProcessExternalTask, disabled: Boolean, onRetry: () -> Unit, onUnlock: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text("${task.topicName} · попыток: ${task.retries ?: "—"}", fontWeight = FontWeight.Medium, fontSize = 12.sp)
            Text(task.errorMessage ?: "Ошибка отсутствует", color = if (task.errorMessage != null) Danger else TextSecondary, fontSize = 11.sp, maxLines = 2)
            Text("обработчик: ${task.workerId ?: "—"} · блокировка до: ${task.lockExpirationTime ?: "—"}", color = TextSecondary, fontSize = 9.sp)
        }
        Button(onClick = onRetry, enabled = !disabled) { Text("Дать 3 попытки") }
        Spacer(Modifier.width(6.dp))
        OutlinedButton(onClick = onUnlock, enabled = !disabled && task.workerId != null) { Text("Разблокировать") }
    }
}

@Composable
private fun VariableRow(variable: ProcessVariable, editEnabled: Boolean, onEdit: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(8.dp),
    ) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 9.dp), verticalAlignment = Alignment.Top) {
            Column(Modifier.width(240.dp)) {
                Text(variable.name, fontWeight = FontWeight.SemiBold, fontFamily = FontFamily.Monospace, fontSize = 11.sp)
                Text(variable.type, color = TextSecondary, fontSize = 9.sp)
            }
            Text(variable.value, modifier = Modifier.weight(1f), fontSize = 10.sp, fontFamily = FontFamily.Monospace, maxLines = 4, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.width(10.dp))
            OutlinedButton(onClick = onEdit, enabled = editEnabled) {
                Text(if (variable.isEditable) "Изменить" else "Только чтение")
            }
        }
    }
}

@Composable
private fun VariableEditDialog(
    variable: ProcessVariable,
    production: Boolean,
    busy: Boolean,
    operationError: String?,
    onDismiss: () -> Unit,
    onSave: (String) -> Unit,
) {
    var newValue by remember(variable.name, variable.value) { mutableStateOf(variable.value) }
    var productionConfirmed by remember(variable.name) { mutableStateOf(false) }
    val validationError = variable.validate(newValue)
    val changed = newValue != variable.value

    DialogWindow(
        onCloseRequest = onDismiss,
        title = "Изменение переменной",
        state = rememberDialogState(width = 760.dp, height = 620.dp),
        resizable = true,
    ) {
        SelectionContainer {
            Card(
                Modifier.fillMaxSize().padding(10.dp),
                colors = CardDefaults.cardColors(containerColor = Surface),
                border = BorderStroke(1.dp, Border),
                shape = RoundedCornerShape(14.dp),
            ) {
                Column(Modifier.fillMaxSize().padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Изменить переменную", fontSize = 20.sp, fontWeight = FontWeight.SemiBold)
                Text(
                    "Значение будет записано непосредственно в активный экземпляр процесса.",
                    color = TextSecondary,
                    fontSize = 12.sp,
                )
                Row(
                    Modifier.fillMaxWidth().background(PrimaryMuted.copy(alpha = 0.45f), RoundedCornerShape(9.dp)).padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text(variable.name, fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Bold)
                        Text("Имя переменной", color = TextSecondary, fontSize = 10.sp)
                    }
                    Column(Modifier.width(180.dp)) {
                        Text(variable.type, fontFamily = FontFamily.Monospace, fontWeight = FontWeight.SemiBold)
                        Text("Тип сохраняется", color = TextSecondary, fontSize = 10.sp)
                    }
                }
                OutlinedTextField(
                    value = newValue,
                    onValueChange = { newValue = it },
                    label = { Text("Новое значение") },
                    supportingText = {
                        Text(validationError ?: variable.editHint, color = if (validationError != null) Danger else TextSecondary)
                    },
                    isError = validationError != null,
                    enabled = !busy,
                    modifier = Modifier.fillMaxWidth().weight(1f),
                    minLines = 8,
                )
                operationError?.let { InlineMessage(it, Danger) }
                if (production) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(
                            checked = productionConfirmed,
                            onCheckedChange = { productionConfirmed = it },
                            enabled = !busy,
                        )
                        Text("Подтверждаю изменение переменной в ПРОДАКШЕНЕ", color = Danger, fontWeight = FontWeight.Bold)
                    }
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    OutlinedButton(onClick = onDismiss, enabled = !busy) { Text("Отмена") }
                    Spacer(Modifier.width(8.dp))
                    Button(
                        onClick = { onSave(newValue) },
                        enabled = !busy && changed && validationError == null && (!production || productionConfirmed),
                    ) {
                        Text(if (busy) "Сохраняем…" else "Сохранить значение")
                    }
                }
                }
            }
        }
    }
}

@Composable
private fun InlineMessage(message: String, color: Color) {
    Text(
        message,
        color = color,
        fontSize = 11.sp,
        modifier = Modifier.fillMaxWidth().background(color.copy(alpha = 0.08f), RoundedCornerShape(7.dp)).padding(horizontal = 10.dp, vertical = 7.dp),
    )
}

@Composable
private fun ConfirmOperationDialog(action: PendingAction, production: Boolean, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    var productionConfirmed by remember { mutableStateOf(false) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { SelectionContainer { Text("Подтверждение операции") } },
        text = {
            SelectionContainer {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(action.description)
                    if (production) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Checkbox(checked = productionConfirmed, onCheckedChange = { productionConfirmed = it })
                            Text("Подтверждаю выполнение операции в ПРОДАКШЕНЕ", color = Danger)
                        }
                    }
                }
            }
        },
        confirmButton = {
            DisableSelection { Button(onClick = onConfirm, enabled = !production || productionConfirmed) { Text("Выполнить") } }
        },
        dismissButton = { DisableSelection { OutlinedButton(onClick = onDismiss) { Text("Отмена") } } },
    )
}

@Composable
private fun TeleportDialog(
    details: ProcessInstanceDetails,
    initialTargetId: String?,
    production: Boolean,
    busy: Boolean,
    onDismiss: () -> Unit,
    onExecute: (TeleportRequest) -> Unit,
) {
    var source by remember { mutableStateOf(details.activeActivities.firstOrNull()) }
    var target by remember(initialTargetId, details.instance.id) {
        mutableStateOf(details.diagram.teleportTargets.firstOrNull { it.id == initialTargetId })
    }
    var sourceQuery by remember { mutableStateOf("") }
    var targetQuery by remember { mutableStateOf("") }
    var annotation by remember { mutableStateOf("") }
    var confirmed by remember { mutableStateOf(false) }
    val sources = details.activeActivities.filter { it.matches(sourceQuery) }
    val targets = details.diagram.teleportTargets.filter { it.matches(targetQuery) }

    DialogWindow(
        onCloseRequest = onDismiss,
        title = "Телепорт заявки",
        state = rememberDialogState(width = 940.dp, height = 760.dp),
        resizable = true,
    ) {
        SelectionContainer {
            Card(
                Modifier.fillMaxSize().padding(10.dp),
                colors = CardDefaults.cardColors(containerColor = Surface),
                border = BorderStroke(1.dp, Border),
                shape = RoundedCornerShape(14.dp),
            ) {
                Column(Modifier.fillMaxSize().padding(18.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Телепорт заявки", fontSize = 20.sp, fontWeight = FontWeight.SemiBold)
                Text("Выберите точный активный экземпляр и кубик, перед которым продолжится выполнение.", color = TextSecondary, fontSize = 12.sp)

                Row(Modifier.weight(1f), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    PickerPanel("1 · ОТКУДА", "Активный элемент", Modifier.weight(1f)) {
                        OutlinedTextField(
                            value = sourceQuery,
                            onValueChange = { sourceQuery = it },
                            placeholder = { Text("Поиск по названию или ID") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
                            items(sources, key = ActiveActivityInstance::id) { activity ->
                                PickerItem(
                                    title = activity.activityName ?: activity.activityId,
                                    subtitle = "${activity.activityType} · ${activity.activityId}",
                                    selected = source?.id == activity.id,
                                    onClick = { source = activity },
                                )
                            }
                        }
                    }
                    PickerPanel("2 · КУДА", "Целевой кубик BPMN", Modifier.weight(1f)) {
                        OutlinedTextField(
                            value = targetQuery,
                            onValueChange = { targetQuery = it },
                            placeholder = { Text("Название, ID или topic кубика") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Text("Найдено: ${targets.size}", color = TextSecondary, fontSize = 10.sp)
                        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
                            items(targets, key = BpmnNode::id) { node ->
                                PickerItem(
                                    title = node.name ?: node.id,
                                    subtitle = listOfNotNull(node.type, node.topic?.let { "topic: $it" }, node.id).joinToString(" · "),
                                    selected = target?.id == node.id,
                                    onClick = { target = node },
                                )
                            }
                        }
                    }
                }

                OutlinedTextField(
                    value = annotation,
                    onValueChange = { annotation = it },
                    label = { Text("Причина или номер обращения") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                Row(
                    Modifier.fillMaxWidth().background(PrimaryMuted.copy(alpha = 0.48f), RoundedCornerShape(8.dp)).padding(10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("${source?.activityName ?: source?.activityId ?: "Источник не выбран"}  →  ${target?.name ?: target?.id ?: "Цель не выбрана"}", fontWeight = FontWeight.SemiBold, fontSize = 11.sp)
                    Spacer(Modifier.weight(1f))
                    Text("cancel + startBeforeActivity", color = TextSecondary, fontFamily = FontFamily.Monospace, fontSize = 9.sp)
                }
                if (production) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = confirmed, onCheckedChange = { confirmed = it })
                        Text("Подтверждаю телепорт в ПРОДАКШЕНЕ", color = Danger, fontWeight = FontWeight.Bold, fontSize = 11.sp)
                    }
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    OutlinedButton(onClick = onDismiss, enabled = !busy) { Text("Отмена") }
                    Spacer(Modifier.width(8.dp))
                    Button(
                        enabled = !busy && source != null && target != null && annotation.isNotBlank() && (!production || confirmed),
                        onClick = {
                            onExecute(
                                TeleportRequest(
                                    sourceActivityInstanceId = source!!.id,
                                    targetActivityId = target!!.id,
                                    annotation = annotation.trim(),
                                ),
                            )
                        },
                    ) { Text(if (busy) "Выполняем…" else "Выполнить телепорт") }
                }
                }
            }
        }
    }
}

@Composable
private fun PickerPanel(title: String, subtitle: String, modifier: Modifier, content: @Composable ColumnScope.() -> Unit) {
    Card(
        modifier = modifier.fillMaxHeight(),
        colors = CardDefaults.cardColors(containerColor = Color(0xFFFAFBFD)),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(10.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(title, color = Primary, fontSize = 10.sp, fontWeight = FontWeight.Bold)
            Text(subtitle, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
            content()
        }
    }
}

@Composable
private fun PickerItem(title: String, subtitle: String, selected: Boolean, onClick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .background(if (selected) PrimaryMuted else Surface, RoundedCornerShape(8.dp))
            .clickable(onClick = onClick)
            .padding(horizontal = 10.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(8.dp).height(8.dp).background(if (selected) Primary else Border, RoundedCornerShape(4.dp)))
        Spacer(Modifier.width(8.dp))
        Column(Modifier.weight(1f)) {
            Text(title, fontSize = 11.sp, fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(subtitle, color = TextSecondary, fontSize = 9.sp, fontFamily = FontFamily.Monospace, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
    }
}

private fun ActiveActivityInstance.matches(query: String): Boolean = query.isBlank() ||
    listOfNotNull(activityName, activityId, activityType, id).any { it.contains(query.trim(), ignoreCase = true) }

private fun BpmnNode.matches(query: String): Boolean = query.isBlank() ||
    listOfNotNull(name, id, type, topic).any { it.contains(query.trim(), ignoreCase = true) }

private val ProcessVariable.isEditable: Boolean
    get() = type.lowercase() !in setOf("object", "file", "bytes", "null")

private val ProcessVariable.editHint: String
    get() = when (type.lowercase()) {
        "boolean" -> "Введите true или false."
        "short", "integer", "long" -> "Введите целое число без пробелов и разделителей."
        "double" -> "Введите число, дробная часть отделяется точкой."
        "date" -> "Сохранится текущий тип Date; используйте формат даты Camunda с часовым поясом."
        "json" -> "Введите полный корректный JSON. Он будет сохранён как строковое JSON-значение Camunda."
        else -> "Тип переменной останется ${type}."
    }

private fun ProcessVariable.validate(newValue: String): String? = when (type.lowercase()) {
    "boolean" -> if (newValue.trim().lowercase() in setOf("true", "false")) null else "Допустимы только true или false."
    "short" -> if (newValue.trim().toShortOrNull() != null) null else "Значение выходит за диапазон Short или не является целым числом."
    "integer" -> if (newValue.trim().toIntOrNull() != null) null else "Значение выходит за диапазон Integer или не является целым числом."
    "long" -> if (newValue.trim().toLongOrNull() != null) null else "Значение выходит за диапазон Long или не является целым числом."
    "double" -> if (newValue.trim().toDoubleOrNull()?.isFinite() == true) null else "Введите конечное число в формате Double."
    else -> null
}

private sealed interface PendingAction {
    val description: String

    data class JobRetry(val jobId: String) : PendingAction {
        override val description = "Установить 3 попытки для задания движка $jobId?"
    }

    data class ExternalRetry(val taskId: String) : PendingAction {
        override val description = "Установить 3 попытки для внешней задачи $taskId?"
    }

    data class ExternalUnlock(val taskId: String) : PendingAction {
        override val description = "Снять блокировку и обработчика с внешней задачи $taskId? Другой обработчик сможет забрать её сразу."
    }

    data class Suspension(val suspended: Boolean) : PendingAction {
        override val description = if (suspended) "Приостановить экземпляр процесса?" else "Активировать экземпляр процесса?"
    }
}
