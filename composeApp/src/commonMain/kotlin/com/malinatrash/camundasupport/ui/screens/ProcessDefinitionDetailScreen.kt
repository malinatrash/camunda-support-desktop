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
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.DisableSelection
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
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
import com.malinatrash.camundasupport.model.ProcessDefinitionDetails
import com.malinatrash.camundasupport.model.ProcessMetadataField
import com.malinatrash.camundasupport.model.RuntimeInstanceListItem
import com.malinatrash.camundasupport.ui.components.EmptyPanel
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.Healthy
import com.malinatrash.camundasupport.ui.theme.PrimaryMuted
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextSecondary
import com.malinatrash.camundasupport.ui.theme.Warning

@Composable
fun ProcessDefinitionDetailScreen(
    connection: CamundaConnection,
    processDefinitionId: String,
    dateFilter: DashboardDateFilter,
    camundaApi: CamundaApi,
    onBack: () -> Unit,
    onOpenInstance: (String) -> Unit,
    onOpenBrowser: () -> Unit,
) {
    var details by remember { mutableStateOf<ProcessDefinitionDetails?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }
    var metadataVisible by remember { mutableStateOf(false) }
    var refreshToken by remember { mutableIntStateOf(0) }

    LaunchedEffect(connection.id, processDefinitionId, dateFilter, refreshToken) {
        loading = true
        error = null
        when (val result = camundaApi.loadProcessDefinitionDetails(connection, processDefinitionId, dateFilter)) {
            is CamundaApiResult.Success -> details = result.value
            is CamundaApiResult.Failure -> error = result.message
        }
        loading = false
    }

    Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            OutlinedButton(onClick = onBack) { Text("← Назад") }
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text(details?.definition?.displayName ?: "Процесс", fontSize = 20.sp, fontWeight = FontWeight.SemiBold)
                Text(processDefinitionId, color = TextSecondary, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
            }
            OutlinedButton(onClick = { refreshToken += 1 }, enabled = !loading) {
                Text(if (loading) "Обновляем…" else "Обновить")
            }
            Spacer(Modifier.width(8.dp))
            OutlinedButton(onClick = onOpenBrowser, enabled = connection.cockpitUrl.isNotBlank()) {
                Text("Открыть в браузере ↗")
            }
        }

        if (details == null && loading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
            return@Column
        }

        error?.let { message ->
            EmptyPanel(
                title = "Не удалось загрузить процесс",
                description = message,
                actionLabel = "Повторить",
                onAction = { refreshToken += 1 },
            )
            if (details == null) return@Column
        }

        val snapshot = details ?: return@Column
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            DefinitionMetric("ВЕРСИЯ", "v${snapshot.definition.version}", Modifier.weight(1f))
            DefinitionMetric(if (dateFilter.isLive) "ЗАПУЩЕНО СЕЙЧАС" else "СТАРТОВАЛО ЗА ПЕРИОД", snapshot.instances.size.toString(), Modifier.weight(1f))
            DefinitionMetric("С ИНЦИДЕНТАМИ", snapshot.instances.count { it.incidentCount > 0 }.toString(), Modifier.weight(1f))
            DefinitionMetric("ТЕНАНТ", snapshot.definition.tenantId ?: "—", Modifier.weight(1f))
        }

        OutlinedButton(onClick = { metadataVisible = !metadataVisible }) {
            Text(if (metadataVisible) "Скрыть метаданные" else "Подробнее о процессе")
        }
        Text(dateFilter.instancesTitle(), fontSize = 15.sp, fontWeight = FontWeight.SemiBold)
        if (snapshot.instances.isEmpty()) {
            EmptyPanel(
                title = if (dateFilter.isLive) "Запущенных экземпляров нет" else "За период заявок нет",
                description = if (dateFilter.isLive) {
                    "Для этой версии процесса Camunda не вернула запущенные экземпляры."
                } else {
                    "За выбранный период этот процесс не запускался."
                },
            )
        } else {
            InstanceList(
                instances = snapshot.instances,
                live = dateFilter.isLive,
                onOpenInstance = onOpenInstance,
                modifier = Modifier.weight(1f),
            )
        }
    }

    if (metadataVisible && details != null) {
        MetadataDialog(fields = details!!.metadata, onDismiss = { metadataVisible = false })
    }
}

@Composable
private fun MetadataDialog(fields: List<ProcessMetadataField>, onDismiss: () -> Unit) {
    DisableSelection {
        AlertDialog(
            onDismissRequest = onDismiss,
            title = { SelectionContainer { Text("Метаданные процесса") } },
            text = {
                SelectionContainer {
                    LazyColumn(verticalArrangement = Arrangement.spacedBy(7.dp)) {
                        items(fields) { field ->
                            Row(Modifier.fillMaxWidth()) {
                                Text(field.label, modifier = Modifier.width(220.dp), color = TextSecondary, fontSize = 11.sp)
                                Text(field.value, modifier = Modifier.weight(1f), fontFamily = FontFamily.Monospace, fontSize = 11.sp)
                            }
                        }
                    }
                }
            },
            confirmButton = { DisableSelection { Button(onClick = onDismiss) { Text("Закрыть") } } },
        )
    }
}

@Composable
private fun DefinitionMetric(label: String, value: String, modifier: Modifier) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(11.dp),
    ) {
        Column(Modifier.padding(horizontal = 12.dp, vertical = 9.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
            Text(label, color = TextSecondary, fontSize = 9.sp, fontWeight = FontWeight.Bold)
            Text(value, fontSize = 17.sp, fontFamily = FontFamily.Monospace, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

@Composable
private fun InstanceList(
    instances: List<RuntimeInstanceListItem>,
    live: Boolean,
    onOpenInstance: (String) -> Unit,
    modifier: Modifier,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(12.dp),
    ) {
        Row(
            Modifier.fillMaxWidth().background(PrimaryMuted.copy(alpha = 0.5f)).padding(horizontal = 16.dp, vertical = 11.dp),
        ) {
            InstanceHeader("СОСТОЯНИЕ", 0.8f)
            InstanceHeader("ИНЦИДЕНТЫ", 0.7f)
            InstanceHeader("БИЗНЕС-КЛЮЧ", 1.4f)
            InstanceHeader("ID ЭКЗЕМПЛЯРА", 1.5f)
            InstanceHeader(if (live) "ТЕНАНТ" else "ВРЕМЯ СТАРТА", 1.1f)
            Spacer(Modifier.weight(0.7f))
        }
        HorizontalDivider(color = Border)
        LazyColumn {
            items(instances, key = { it.instance.id }) { item ->
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 9.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box(Modifier.weight(0.8f)) {
                        val color = when {
                            item.instance.ended -> TextSecondary
                            item.instance.suspended -> Warning
                            else -> Healthy
                        }
                        Text(
                            when {
                                item.instance.ended -> "ЗАВЕРШЁН"
                                item.instance.suspended -> "ПРИОСТАНОВЛЕН"
                                else -> "ЗАПУЩЕН"
                            },
                            color = color,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                        )
                    }
                    Box(Modifier.weight(0.7f)) {
                        Text(
                            item.incidentCount.toString(),
                            color = if (item.incidentCount > 0) Danger else TextSecondary,
                            fontWeight = if (item.incidentCount > 0) FontWeight.Bold else FontWeight.Normal,
                        )
                    }
                    Box(Modifier.weight(1.4f)) {
                        Text(item.instance.businessKey ?: "—", maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                    Box(Modifier.weight(1.5f)) {
                        Text(item.instance.id, fontFamily = FontFamily.Monospace, fontSize = 10.sp, maxLines = 1)
                    }
                    Box(Modifier.weight(1.1f)) {
                        Text(if (live) item.instance.tenantId ?: "—" else item.instance.startTime ?: "—", color = TextSecondary, maxLines = 1, fontSize = 10.sp)
                    }
                    Box(Modifier.weight(0.7f), contentAlignment = Alignment.CenterEnd) {
                        Button(onClick = { onOpenInstance(item.instance.id) }, enabled = !item.instance.ended) {
                            Text(if (item.instance.ended) "Завершена" else "Подробнее")
                        }
                    }
                }
                HorizontalDivider(color = Border.copy(alpha = 0.65f))
            }
        }
    }
}

private fun DashboardDateFilter.instancesTitle(): String = when (preset) {
    com.malinatrash.camundasupport.model.DashboardDatePreset.Live -> "Заявки, запущенные сейчас"
    com.malinatrash.camundasupport.model.DashboardDatePreset.Today -> "Заявки, стартовавшие сегодня"
    com.malinatrash.camundasupport.model.DashboardDatePreset.Yesterday -> "Заявки, стартовавшие вчера"
    com.malinatrash.camundasupport.model.DashboardDatePreset.Custom -> "Заявки за период $fromDate — $toDate"
}

@Composable
private fun androidx.compose.foundation.layout.RowScope.InstanceHeader(label: String, weight: Float) {
    Text(
        label,
        modifier = Modifier.weight(weight),
        color = TextSecondary,
        fontSize = 9.sp,
        fontWeight = FontWeight.Bold,
    )
}
