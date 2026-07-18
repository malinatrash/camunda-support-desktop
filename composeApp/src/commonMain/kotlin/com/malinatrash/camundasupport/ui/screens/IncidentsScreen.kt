package com.malinatrash.camundasupport.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.text.selection.DisableSelection
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.DashboardDatePreset
import com.malinatrash.camundasupport.model.ProcessIncident
import com.malinatrash.camundasupport.ui.components.EmptyPanel
import com.malinatrash.camundasupport.ui.components.SectionTitle
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.PrimaryMuted
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun IncidentsScreen(
    connection: CamundaConnection?,
    camundaApi: CamundaApi,
    processDefinitionId: String?,
    dateFilter: DashboardDateFilter,
    onAddConnection: () -> Unit,
    onOpenInstance: (String) -> Unit,
) {
    var incidents by remember { mutableStateOf<List<ProcessIncident>>(emptyList()) }
    var loading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var query by remember { mutableStateOf("") }
    var selectedType by remember { mutableStateOf<String?>(null) }
    var refreshToken by remember { mutableIntStateOf(0) }
    var appliedDateFilter by remember(dateFilter) { mutableStateOf(dateFilter) }
    var selectedDatePreset by remember(dateFilter) { mutableStateOf(dateFilter.preset) }
    var customFromDate by remember(dateFilter) { mutableStateOf(dateFilter.fromDate) }
    var customToDate by remember(dateFilter) { mutableStateOf(dateFilter.toDate) }
    var selectedDefinitionId by remember(processDefinitionId) { mutableStateOf(processDefinitionId) }
    var versions by remember { mutableStateOf<List<IncidentVersionOption>>(emptyList()) }
    var versionsLoading by remember { mutableStateOf(false) }
    var versionMenuExpanded by remember { mutableStateOf(false) }
    val processDefinitionKey = remember(processDefinitionId) { processDefinitionId?.definitionKey() }

    LaunchedEffect(connection?.id, processDefinitionKey) {
        versions = emptyList()
        if (connection == null || processDefinitionKey == null) return@LaunchedEffect
        versionsLoading = true
        when (val result = camundaApi.loadProcessDefinitionVersions(connection, processDefinitionKey)) {
            is CamundaApiResult.Success -> versions = result.value.map { definition ->
                IncidentVersionOption(
                    definitionId = definition.id,
                    processKey = definition.key,
                    version = definition.version,
                    versionTag = definition.versionTag,
                )
            }
            is CamundaApiResult.Failure -> Unit
        }
        versionsLoading = false
    }

    LaunchedEffect(
        connection?.id,
        selectedDefinitionId,
        processDefinitionKey,
        appliedDateFilter,
        refreshToken,
    ) {
        if (connection == null) return@LaunchedEffect
        loading = true
        error = null
        when (val result = camundaApi.loadIncidents(
            connection = connection,
            processDefinitionId = selectedDefinitionId,
            dateFilter = appliedDateFilter,
            processDefinitionKey = processDefinitionKey.takeIf { selectedDefinitionId == null },
        )) {
            is CamundaApiResult.Success -> incidents = result.value
            is CamundaApiResult.Failure -> error = result.message
        }
        loading = false
    }

    Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(verticalAlignment = Alignment.Bottom) {
            SectionTitle(
                title = if (processDefinitionId == null) "Инциденты" else "Инциденты процесса",
                description = buildString {
                    append(if (appliedDateFilter.isLive) "Открытые инциденты Camunda" else "История инцидентов: ${appliedDateFilter.preset.label.lowercase()}")
                    processDefinitionKey?.let { append(" · $it") }
                },
                modifier = Modifier.weight(1f),
            )
            if (connection != null) {
                OutlinedButton(onClick = { refreshToken += 1 }, enabled = !loading) {
                    Text(if (loading) "Обновляем…" else "Обновить")
                }
            }
        }

        if (connection == null) {
            EmptyPanel(
                title = "Требуется подключение",
                description = "Добавьте подключение Camunda, чтобы просматривать инциденты.",
                actionLabel = "Добавить подключение",
                onAction = onAddConnection,
            )
            return@Column
        }

        error?.let {
            EmptyPanel(
                title = "Не удалось загрузить инциденты",
                description = it,
                actionLabel = "Повторить",
                onAction = { refreshToken += 1 },
            )
            if (incidents.isEmpty()) return@Column
        }

        if (loading && incidents.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
            return@Column
        }

        val types = incidents.groupingBy(ProcessIncident::type).eachCount().entries.sortedByDescending { it.value }
        val incidentVersions = incidents.mapNotNull { incident ->
            incident.processDefinitionId?.toIncidentVersionOption()
        }.distinctBy(IncidentVersionOption::definitionId)
        val versionOptions = (versions.ifEmpty { incidentVersions }).sortedByDescending(IncidentVersionOption::version)
        val selectedVersionLabel = selectedDefinitionId?.let { selectedId ->
            versionOptions.firstOrNull { it.definitionId == selectedId }?.label(scoped = processDefinitionKey != null)
                ?: selectedId.toIncidentVersionOption()?.label(scoped = processDefinitionKey != null)
        } ?: if (processDefinitionKey == null) "Все процессы и версии" else "Все версии процесса"
        val filtered = incidents.filter { incident ->
            (selectedType == null || incident.type == selectedType) &&
                (query.isBlank() || listOfNotNull(
                    incident.id,
                    incident.message,
                    incident.activityId,
                    incident.failedActivityId,
                    incident.processInstanceId,
                    incident.processDefinitionId,
                    incident.configuration,
                ).any { it.contains(query.trim(), ignoreCase = true) })
        }

        Card(
            colors = CardDefaults.cardColors(containerColor = Surface),
            border = BorderStroke(1.dp, Border),
            shape = RoundedCornerShape(12.dp),
        ) {
            Column(Modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(9.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text("Период", color = TextSecondary, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    DashboardDatePreset.entries.forEach { preset ->
                        IncidentTypeButton(
                            label = if (preset == DashboardDatePreset.Live) "Открытые" else preset.label,
                            selected = selectedDatePreset == preset,
                        ) {
                            selectedDatePreset = preset
                            if (preset != DashboardDatePreset.Custom) {
                                appliedDateFilter = DashboardDateFilter(preset = preset)
                            }
                        }
                    }
                    Spacer(Modifier.weight(1f))
                    Text("Версия", color = TextSecondary, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    Box {
                        OutlinedButton(
                            onClick = { versionMenuExpanded = true },
                            enabled = !versionsLoading,
                        ) {
                            Text(if (versionsLoading) "Загружаем версии…" else "$selectedVersionLabel  ▾")
                        }
                        DisableSelection {
                            DropdownMenu(
                                expanded = versionMenuExpanded,
                                onDismissRequest = { versionMenuExpanded = false },
                            ) {
                                SelectionContainer {
                                    Column {
                                    DropdownMenuItem(
                                        text = { Text(if (processDefinitionKey == null) "Все процессы и версии" else "Все версии процесса") },
                                        onClick = {
                                            selectedDefinitionId = null
                                            versionMenuExpanded = false
                                        },
                                    )
                                    versionOptions.forEach { option ->
                                DropdownMenuItem(
                                    text = {
                                        Column {
                                            Text(option.label(scoped = processDefinitionKey != null))
                                            Text(option.definitionId, color = TextSecondary, fontSize = 9.sp, fontFamily = FontFamily.Monospace)
                                        }
                                    },
                                    onClick = {
                                        selectedDefinitionId = option.definitionId
                                        versionMenuExpanded = false
                                    },
                                )
                                    }
                                    }
                                }
                            }
                        }
                    }
                }
                if (selectedDatePreset == DashboardDatePreset.Custom) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = customFromDate,
                            onValueChange = { customFromDate = it },
                            label = { Text("Дата с") },
                            placeholder = { Text("ГГГГ-ММ-ДД") },
                            singleLine = true,
                            modifier = Modifier.width(180.dp),
                        )
                        OutlinedTextField(
                            value = customToDate,
                            onValueChange = { customToDate = it },
                            label = { Text("Дата по") },
                            placeholder = { Text("ГГГГ-ММ-ДД") },
                            singleLine = true,
                            modifier = Modifier.width(180.dp),
                        )
                        Button(onClick = {
                            appliedDateFilter = DashboardDateFilter(
                                preset = DashboardDatePreset.Custom,
                                fromDate = customFromDate,
                                toDate = customToDate,
                            )
                        }) {
                            Text("Применить период")
                        }
                    }
                }
                HorizontalDivider(color = Border)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Всего: ${incidents.size}", fontWeight = FontWeight.Bold)
                    Spacer(Modifier.width(16.dp))
                    OutlinedTextField(
                        value = query,
                        onValueChange = { query = it },
                        placeholder = { Text("Сообщение, заявка, кубик или ID") },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                    )
                }
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    IncidentTypeButton("Все · ${incidents.size}", selectedType == null) { selectedType = null }
                    types.take(6).forEach { (type, count) ->
                        IncidentTypeButton("${typeLabel(type)} · $count", selectedType == type) { selectedType = type }
                    }
                }
            }
        }

        if (filtered.isEmpty()) {
            EmptyPanel("Инциденты не найдены", "Измените фильтр или обновите данные.")
        } else {
            LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(filtered, key = ProcessIncident::id) { incident ->
                    IncidentCard(incident, onOpenInstance)
                }
            }
        }
    }
}

@Composable
private fun IncidentTypeButton(label: String, selected: Boolean, onClick: () -> Unit) {
    OutlinedButton(
        onClick = onClick,
        border = BorderStroke(1.dp, if (selected) Primary else Border),
    ) {
        Text(label, color = if (selected) Primary else TextSecondary, fontSize = 10.sp)
    }
}

@Composable
private fun IncidentCard(incident: ProcessIncident, onOpenInstance: (String) -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Danger.copy(alpha = 0.35f)),
        shape = RoundedCornerShape(11.dp),
    ) {
        Row(Modifier.fillMaxWidth().padding(13.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        typeLabel(incident.type).uppercase(),
                        color = Danger,
                        fontSize = 9.sp,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.background(Danger.copy(alpha = 0.09f), RoundedCornerShape(6.dp)).padding(horizontal = 7.dp, vertical = 4.dp),
                    )
                    Spacer(Modifier.width(8.dp))
                    Text(incident.timestamp ?: "Время неизвестно", color = TextSecondary, fontSize = 10.sp)
                }
                Text(incident.message ?: "Camunda не вернула сообщение ошибки", fontWeight = FontWeight.SemiBold, maxLines = 2, overflow = TextOverflow.Ellipsis)
                Text(
                    listOfNotNull(
                        incident.failedActivityId?.let { "кубик: $it" },
                        incident.processDefinitionId?.toIncidentVersionOption()?.let {
                            "процесс: ${it.processKey} · v${it.version}"
                        },
                        incident.processInstanceId?.let { "заявка: $it" },
                        incident.tenantId?.let { "тенант: $it" },
                    ).joinToString(" · ").ifBlank { "Технические связи отсутствуют" },
                    color = TextSecondary,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 9.sp,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                incident.annotation?.takeIf(String::isNotBlank)?.let {
                    Text("Комментарий: $it", color = TextSecondary, fontSize = 10.sp)
                }
            }
            val instanceId = incident.processInstanceId
            OutlinedButton(onClick = { onOpenInstance(instanceId!!) }, enabled = !instanceId.isNullOrBlank()) {
                Text("Открыть заявку")
            }
        }
    }
}

private fun typeLabel(type: String): String = when (type) {
    "failedJob" -> "Ошибка задания"
    "failedExternalTask" -> "Ошибка внешней задачи"
    else -> type
}

private data class IncidentVersionOption(
    val definitionId: String,
    val processKey: String,
    val version: Int,
    val versionTag: String? = null,
) {
    fun label(scoped: Boolean): String = buildString {
        if (!scoped) append("$processKey · ")
        append("Версия $version")
        versionTag?.takeIf(String::isNotBlank)?.let { append(" · $it") }
    }
}

private fun String.definitionKey(): String? = substringBefore(':').takeIf(String::isNotBlank)

private fun String.toIncidentVersionOption(): IncidentVersionOption? {
    val parts = split(':', limit = 3)
    val key = parts.getOrNull(0)?.takeIf(String::isNotBlank) ?: return null
    val version = parts.getOrNull(1)?.toIntOrNull() ?: return null
    return IncidentVersionOption(
        definitionId = this,
        processKey = key,
        version = version,
    )
}
