package com.malinatrash.camundasupport.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.text.selection.DisableSelection
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.onPointerEvent
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import kotlin.math.max
import kotlin.math.roundToInt
import kotlinx.coroutines.launch
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.DashboardDatePreset
import com.malinatrash.camundasupport.model.DashboardIncidentActivity
import com.malinatrash.camundasupport.model.DashboardDurationBucket
import com.malinatrash.camundasupport.model.DeploymentSort
import com.malinatrash.camundasupport.model.ProcessDashboard
import com.malinatrash.camundasupport.state.DashboardPollingStore
import com.malinatrash.camundasupport.model.ProcessDefinitionSummary
import com.malinatrash.camundasupport.model.DashboardTimelinePoint
import com.malinatrash.camundasupport.model.DashboardWaitingActivity
import com.malinatrash.camundasupport.ui.components.EmptyPanel
import com.malinatrash.camundasupport.ui.components.SectionTitle
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.Healthy
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.PrimaryMuted
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary
import com.malinatrash.camundasupport.ui.theme.Warning

private enum class DefinitionView { List, Previews }

@Composable
internal fun OverviewScreen(
    connection: CamundaConnection?,
    dashboardStore: DashboardPollingStore,
    onAddConnection: () -> Unit,
    onOpenDefinition: (String, DashboardDateFilter) -> Unit,
    onOpenIncidents: (String?, DashboardDateFilter) -> Unit,
    onOpenInstance: (String) -> Unit,
) {
    var sort by remember { mutableStateOf(DeploymentSort.NewestFirst) }
    var view by remember { mutableStateOf(DefinitionView.List) }
    var query by remember { mutableStateOf("") }
    var selectedDatePreset by remember { mutableStateOf(DashboardDatePreset.Live) }
    var appliedDateFilter by remember { mutableStateOf(DashboardDateFilter()) }
    var customFromDate by remember { mutableStateOf("") }
    var customToDate by remember { mutableStateOf("") }
    val pageScrollState = rememberScrollState()
    val coroutineScope = rememberCoroutineScope()
    var processSectionOffset by remember { mutableIntStateOf(0) }

    SideEffect { dashboardStore.activate(connection, appliedDateFilter) }
    val cacheEntry = connection?.let { dashboardStore.entry(it, appliedDateFilter) }
    val dashboard = cacheEntry?.dashboard?.let { cached ->
        if (sort == DeploymentSort.NewestFirst) cached else cached.copy(definitions = cached.definitions.asReversed())
    }
    val isLoading = cacheEntry?.isRefreshing == true
    val error = cacheEntry?.error

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(pageScrollState).padding(end = 4.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(verticalAlignment = Alignment.Bottom) {
            SectionTitle(
                title = "Дашборд подключения",
                description = "Последние версии процессов, экземпляры и инциденты Camunda.",
                modifier = Modifier.weight(1f),
            )
            if (connection != null) {
                OutlinedButton(
                    onClick = {
                        coroutineScope.launch {
                            pageScrollState.animateScrollTo(processSectionOffset.coerceAtLeast(0))
                        }
                    },
                    enabled = processSectionOffset > 0,
                ) {
                    Text("К процессам ↓")
                }
                Spacer(Modifier.width(8.dp))
                Text(
                    if (isLoading) "Автообновление…"
                    else "Автообновление через: ${dashboardStore.secondsUntilRefresh} сек",
                    color = TextSecondary,
                    fontSize = 10.sp,
                )
                Spacer(Modifier.width(8.dp))
                OutlinedButton(
                    onClick = { coroutineScope.launch { dashboardStore.refreshActive(force = true) } },
                    enabled = !isLoading,
                ) {
                    Text(if (isLoading) "Обновляем…" else "Обновить")
                }
            }
        }

        if (connection == null) {
            EmptyPanel(
                title = "Добавьте первое окружение Camunda",
                description = "Укажите адрес REST API, чтобы загрузить развёрнутые процессы.",
                actionLabel = "Добавить подключение",
                onAction = onAddConnection,
            )
            return@Column
        }

        if (dashboard == null && isLoading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                CircularProgressIndicator()
            }
            return@Column
        }

        error?.let { message ->
            EmptyPanel(
                title = "Не удалось загрузить дашборд",
                description = message,
                actionLabel = "Повторить",
                onAction = { coroutineScope.launch { dashboardStore.refreshActive(force = true) } },
            )
            if (dashboard == null) return@Column
        }

        val snapshot = dashboard ?: return@Column
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            MetricCard("ВЕРСИЯ CAMUNDA", connection.engineVersion ?: "Подключено", "GET /version", Healthy, Modifier.weight(1f))
            MetricCard("ОПРЕДЕЛЕНИЯ ПРОЦЕССОВ", snapshot.definitions.size.toString(), "Последние версии", Primary, Modifier.weight(1f))
            MetricCard(
                if (appliedDateFilter.isLive) "ЗАПУЩЕНО СЕЙЧАС" else "СТАРТОВАЛО",
                snapshot.instances.toString(),
                if (appliedDateFilter.isLive) "Текущее состояние" else "За выбранный период",
                Healthy,
                Modifier.weight(1f),
            )
            MetricCard(
                "ЗАВЕРШЕНО ПОЛНОСТЬЮ",
                snapshot.completedInstances.toString(),
                if (appliedDateFilter.isLive) "За последние 24 часа" else "За выбранный период",
                Healthy,
                Modifier.weight(1f),
            )
            MetricCard(
                if (appliedDateFilter.isLive) "ОТКРЫТЫЕ ИНЦИДЕНТЫ" else "СОЗДАНО ИНЦИДЕНТОВ",
                snapshot.incidents.toString(),
                if (appliedDateFilter.isLive) "Текущее состояние" else "За выбранный период",
                if (snapshot.incidents > 0) Danger else Healthy,
                Modifier.weight(1f),
            )
        }

        DateFilterBar(
            selectedPreset = selectedDatePreset,
            customFromDate = customFromDate,
            customToDate = customToDate,
            onPresetSelected = { preset ->
                selectedDatePreset = preset
                if (preset != DashboardDatePreset.Custom) {
                    appliedDateFilter = DashboardDateFilter(preset = preset)
                }
            },
            onFromDateChange = { customFromDate = it },
            onToDateChange = { customToDate = it },
            onApplyCustom = {
                appliedDateFilter = DashboardDateFilter(
                    preset = DashboardDatePreset.Custom,
                    fromDate = customFromDate,
                    toDate = customToDate,
                )
            },
        )

        DashboardAnalytics(
            dashboard = snapshot,
            onOpenIncidents = { definitionId -> onOpenIncidents(definitionId, appliedDateFilter) },
            onOpenInstance = onOpenInstance,
        )

        Row(
            modifier = Modifier.onGloballyPositioned { coordinates ->
                processSectionOffset = coordinates.positionInRoot().y.roundToInt() + pageScrollState.value - 12
            },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(
                    "Развёрнутые процессы: ${snapshot.definitions.size}",
                    fontWeight = FontWeight.SemiBold,
                    fontSize = 16.sp,
                )
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Показаны только последние версии", color = TextSecondary, fontSize = 11.sp)
                    Spacer(Modifier.width(10.dp))
                    Text(
                        "К аналитике ↑",
                        color = Primary,
                        fontSize = 10.sp,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.clickable {
                            coroutineScope.launch { pageScrollState.animateScrollTo(0) }
                        },
                    )
                }
            }
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text("Фильтр процессов") },
                placeholder = { Text("Название, ключ, ID или ID тенанта") },
                singleLine = true,
                modifier = Modifier.width(300.dp),
            )
            Spacer(Modifier.width(10.dp))
            OutlinedButton(
                onClick = {
                    sort = if (sort == DeploymentSort.NewestFirst) {
                        DeploymentSort.OldestFirst
                    } else {
                        DeploymentSort.NewestFirst
                    }
                },
            ) {
                Text(if (sort == DeploymentSort.NewestFirst) "Сначала новые ↓" else "Сначала старые ↑")
            }
            Spacer(Modifier.width(10.dp))
            ViewButton("Список", view == DefinitionView.List) { view = DefinitionView.List }
            Spacer(Modifier.width(6.dp))
            ViewButton("Превью", view == DefinitionView.Previews) { view = DefinitionView.Previews }
        }

        val filtered = snapshot.definitions.filter { definition ->
            query.isBlank() || listOf(
                definition.displayName,
                definition.key,
                definition.id,
                definition.tenantId.orEmpty(),
            ).any { it.contains(query.trim(), ignoreCase = true) }
        }

        if (filtered.isEmpty()) {
            EmptyPanel(
                title = "Процессы не найдены",
                description = "Измените фильтр или обновите дашборд.",
            )
        } else if (view == DefinitionView.List) {
            DefinitionsTable(
                definitions = filtered,
                canOpenCockpit = true,
                liveStatistics = appliedDateFilter.isLive,
                onOpenDefinition = { definitionId -> onOpenDefinition(definitionId, appliedDateFilter) },
                onOpenIncidents = { definitionId -> onOpenIncidents(definitionId, appliedDateFilter) },
                modifier = Modifier.fillMaxWidth(),
            )
        } else {
            DefinitionPreviews(
                definitions = filtered,
                canOpenCockpit = true,
                liveStatistics = appliedDateFilter.isLive,
                onOpenDefinition = { definitionId -> onOpenDefinition(definitionId, appliedDateFilter) },
                onOpenIncidents = { definitionId -> onOpenIncidents(definitionId, appliedDateFilter) },
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

@Composable
private fun DateFilterBar(
    selectedPreset: DashboardDatePreset,
    customFromDate: String,
    customToDate: String,
    onPresetSelected: (DashboardDatePreset) -> Unit,
    onFromDateChange: (String) -> Unit,
    onToDateChange: (String) -> Unit,
    onApplyCustom: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(12.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text("Статистика:", color = TextSecondary, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
            DashboardDatePreset.entries.forEach { preset ->
                ViewButton(
                    label = preset.label,
                    selected = selectedPreset == preset,
                    onClick = { onPresetSelected(preset) },
                )
            }
            if (selectedPreset == DashboardDatePreset.Custom) {
                Spacer(Modifier.width(8.dp))
                OutlinedTextField(
                    value = customFromDate,
                    onValueChange = onFromDateChange,
                    label = { Text("С") },
                    placeholder = { Text("ГГГГ-ММ-ДД") },
                    singleLine = true,
                    modifier = Modifier.width(160.dp),
                )
                OutlinedTextField(
                    value = customToDate,
                    onValueChange = onToDateChange,
                    label = { Text("По") },
                    placeholder = { Text("ГГГГ-ММ-ДД") },
                    singleLine = true,
                    modifier = Modifier.width(160.dp),
                )
                Button(onClick = onApplyCustom) { Text("Применить") }
            }
            Spacer(Modifier.weight(1f))
            Text(
                if (selectedPreset == DashboardDatePreset.Live) {
                    "Текущие данные движка"
                } else {
                    "При открытии процесса покажем заявки за этот период"
                },
                color = TextSecondary,
                fontSize = 10.sp,
            )
        }
    }
}

@Composable
private fun DashboardAnalytics(
    dashboard: ProcessDashboard,
    onOpenIncidents: (String?) -> Unit,
    onOpenInstance: (String) -> Unit,
) {
    val started = dashboard.timeline.sumOf(DashboardTimelinePoint::started)
    val completionRate = if (started == 0) 0.0 else dashboard.completedInstances * 100.0 / started
    var chartExpanded by remember { mutableStateOf(false) }

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        if (chartExpanded) {
            TimelineChart(
                dashboard = dashboard,
                expanded = true,
                onToggleExpanded = { chartExpanded = false },
                modifier = Modifier.fillMaxWidth().height(470.dp),
            )
        } else {
            Row(
                modifier = Modifier.fillMaxWidth().height(340.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                TimelineChart(
                    dashboard = dashboard,
                    expanded = false,
                    onToggleExpanded = { chartExpanded = true },
                    modifier = Modifier.weight(2.25f),
                )
                Column(
                    modifier = Modifier.weight(0.95f),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    IncidentBreakdownCard(dashboard, onOpenIncidents, Modifier.weight(1f))
                    CriticalProcessesCard(dashboard, onOpenIncidents, Modifier.weight(1f))
                }
            }
        }
        TopIncidentActivitiesCard(
            activities = dashboard.incidentActivities,
            onOpenIncidents = onOpenIncidents,
            modifier = Modifier.fillMaxWidth().height(108.dp),
        )
        LongestWaitingActivitiesCard(
            activities = dashboard.longestWaitingActivities,
            onOpenInstance = onOpenInstance,
            modifier = Modifier.fillMaxWidth().height(300.dp),
        )
        DurationDistributionCard(
            buckets = dashboard.durationBuckets,
            averageDurationMillis = dashboard.averageDurationMillis,
            medianDurationMillis = dashboard.medianDurationMillis,
            p95DurationMillis = dashboard.p95DurationMillis,
            modifier = Modifier.fillMaxWidth().height(190.dp),
        )
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            HealthIndicator(
                "СРЕДНЕЕ ВРЕМЯ ПРОХОЖДЕНИЯ",
                formatDuration(dashboard.averageDurationMillis),
                "Для завершённых заявок",
                Primary,
                Modifier.weight(1f),
            )
            HealthIndicator(
                "ДОЛЯ ЗАВЕРШЕНИЯ",
                "${completionRate.roundToInt()}%",
                "${dashboard.completedInstances} из $started стартовавших",
                Healthy,
                Modifier.weight(1f),
            )
            HealthIndicator(
                "ЗАТРОНУТО ПРОЦЕССОВ",
                dashboard.affectedDefinitions.toString(),
                "Из ${dashboard.definitions.size} определений",
                if (dashboard.affectedDefinitions > 0) Danger else Healthy,
                Modifier.weight(1f),
            )
            HealthIndicator(
                "ПРИОСТАНОВЛЕНО",
                dashboard.suspendedDefinitions.toString(),
                "Определений процессов",
                if (dashboard.suspendedDefinitions > 0) Warning else Healthy,
                Modifier.weight(1f),
            )
        }
        if (dashboard.timelineTruncated) {
            Text(
                "На графике показаны первые 20 000 записей каждого типа. Итоговые счётчики остаются точными.",
                color = Warning,
                fontSize = 10.sp,
            )
        }
    }
}

@Composable
private fun LongestWaitingActivitiesCard(
    activities: List<DashboardWaitingActivity>,
    onOpenInstance: (String) -> Unit,
    modifier: Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(12.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(horizontal = 14.dp, vertical = 11.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("Кубики, на которых заявки висят дольше всего", fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                    Text(
                        "Текущие незавершённые кубики · только заявки из выбранного периода",
                        color = TextSecondary,
                        fontSize = 9.sp,
                    )
                }
                Text("Нажмите строку, чтобы открыть самую старую заявку", color = Primary, fontSize = 9.sp)
            }
            HorizontalDivider(color = Border)
            if (activities.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("Незавершённых кубиков с доступной историей нет", color = Healthy, fontSize = 11.sp)
                }
            } else {
                Row(Modifier.fillMaxWidth().padding(horizontal = 9.dp, vertical = 2.dp)) {
                    HeaderCell("КУБИК", 2.4f)
                    HeaderCell("ПРОЦЕСС", 1.45f)
                    HeaderCell("ЗАЯВОК", 0.65f)
                    HeaderCell("СРЕДНЕЕ", 0.9f)
                    HeaderCell("МАКСИМУМ", 0.9f)
                    HeaderCell("САМАЯ СТАРАЯ ЗАЯВКА", 1.5f)
                }
                activities.take(6).forEach { activity ->
                    WaitingActivityRow(activity, onOpenInstance)
                }
            }
        }
    }
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun WaitingActivityRow(activity: DashboardWaitingActivity, onOpenInstance: (String) -> Unit) {
    var hovered by remember(activity.activityId, activity.processDefinitionId) { mutableStateOf(false) }
    val riskColor = when {
        activity.maximumWaitingMillis >= 24 * 60 * 60 * 1_000L -> Danger
        activity.maximumWaitingMillis >= 4 * 60 * 60 * 1_000L -> Warning
        else -> Primary
    }
    Row(
        Modifier.fillMaxWidth()
            .background(if (hovered) riskColor.copy(alpha = 0.09f) else Color.Transparent, RoundedCornerShape(7.dp))
            .onPointerEvent(PointerEventType.Enter) { hovered = true }
            .onPointerEvent(PointerEventType.Exit) { hovered = false }
            .clickable { onOpenInstance(activity.oldestProcessInstanceId) }
            .padding(horizontal = 9.dp, vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TableCell(2.4f) {
            Column {
                Text(
                    activity.activityName ?: activity.topic ?: activity.activityId,
                    fontSize = 10.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    listOfNotNull(activity.activityId, activity.topic?.let { "topic: $it" }).joinToString(" · "),
                    color = TextSecondary,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 8.sp,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        TableCell(1.45f) {
            Text(shortDefinitionLabel(activity.processDefinitionId), fontFamily = FontFamily.Monospace, fontSize = 9.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        TableCell(0.65f) {
            Text(activity.instanceCount.toString(), fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, fontSize = 11.sp)
        }
        TableCell(0.9f) {
            Text(formatDuration(activity.averageWaitingMillis), fontFamily = FontFamily.Monospace, fontSize = 9.sp)
        }
        TableCell(0.9f) {
            Text(formatDuration(activity.maximumWaitingMillis), color = riskColor, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, fontSize = 10.sp)
        }
        TableCell(1.5f) {
            Column {
                Text(activity.oldestProcessInstanceId, color = Primary, fontFamily = FontFamily.Monospace, fontSize = 8.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(activity.oldestStartedAt ?: "Время старта неизвестно", color = TextSecondary, fontSize = 8.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable
private fun DurationDistributionCard(
    buckets: List<DashboardDurationBucket>,
    averageDurationMillis: Long?,
    medianDurationMillis: Long?,
    p95DurationMillis: Long?,
    modifier: Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(12.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(horizontal = 14.dp, vertical = 11.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("Распределение времени прохождения", fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                    Text("Помогает увидеть длинный хвост и заявки, выходящие за ожидаемое время", color = TextSecondary, fontSize = 9.sp)
                }
                DurationMetric("СРЕДНЕЕ", averageDurationMillis, Primary)
                Spacer(Modifier.width(18.dp))
                DurationMetric("МЕДИАНА", medianDurationMillis, Healthy)
                Spacer(Modifier.width(18.dp))
                DurationMetric("P95", p95DurationMillis, Warning)
            }
            HorizontalDivider(color = Border.copy(alpha = 0.7f))
            if (buckets.sumOf(DashboardDurationBucket::count) == 0) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("Нет завершённых заявок с данными о длительности", color = TextSecondary, fontSize = 11.sp)
                }
            } else {
                val maximum = buckets.maxOfOrNull(DashboardDurationBucket::count)?.coerceAtLeast(1) ?: 1
                Row(
                    modifier = Modifier.fillMaxWidth().weight(1f),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalAlignment = Alignment.Bottom,
                ) {
                    buckets.forEachIndexed { index, bucket ->
                        Column(
                            modifier = Modifier.weight(1f).fillMaxHeight(),
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            Text(
                                bucket.count.toString(),
                                color = if (index >= buckets.lastIndex - 1 && bucket.count > 0) Warning else TextPrimary,
                                fontWeight = FontWeight.Bold,
                                fontFamily = FontFamily.Monospace,
                                fontSize = 11.sp,
                            )
                            Box(
                                modifier = Modifier.weight(1f).fillMaxWidth(),
                                contentAlignment = Alignment.BottomCenter,
                            ) {
                                val fraction = bucket.count / maximum.toFloat()
                                Box(
                                    Modifier.width(30.dp)
                                        .fillMaxHeight(fraction)
                                        .background(
                                            when (index) {
                                                buckets.lastIndex -> Danger
                                                buckets.lastIndex - 1 -> Warning
                                                else -> Primary.copy(alpha = 0.82f)
                                            },
                                            RoundedCornerShape(topStart = 5.dp, topEnd = 5.dp),
                                        ),
                                )
                            }
                            Text(bucket.label, color = TextSecondary, fontSize = 9.sp, maxLines = 1, textAlign = TextAlign.Center)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun DurationMetric(label: String, durationMillis: Long?, color: Color) {
    Column(horizontalAlignment = Alignment.End) {
        Text(label, color = TextSecondary, fontSize = 8.sp, fontWeight = FontWeight.Bold)
        Text(formatDuration(durationMillis), color = color, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, fontSize = 12.sp)
    }
}

@Composable
private fun TopIncidentActivitiesCard(
    activities: List<DashboardIncidentActivity>,
    onOpenIncidents: (String?) -> Unit,
    modifier: Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(12.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(horizontal = 13.dp, vertical = 10.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Топ кубиков по инцидентам", fontWeight = FontWeight.SemiBold, fontSize = 12.sp)
                Spacer(Modifier.width(8.dp))
                Text("Название из BPMN-схемы", color = TextSecondary, fontSize = 8.sp)
                Spacer(Modifier.weight(1f))
                Text("Нажмите, чтобы открыть инциденты процесса", color = TextSecondary, fontSize = 9.sp)
            }
            if (activities.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("Проблемных кубиков нет", color = Healthy, fontSize = 11.sp)
                }
            } else {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    activities.take(6).forEachIndexed { index, activity ->
                        val processLabel = activity.processDefinitionId?.let(::shortDefinitionLabel) ?: "Процесс неизвестен"
                        Column(
                            modifier = Modifier.weight(1f)
                                .background(Danger.copy(alpha = 0.07f), RoundedCornerShape(8.dp))
                                .then(
                                    activity.processDefinitionId?.let { definitionId ->
                                        Modifier.clickable { onOpenIncidents(definitionId) }
                                    } ?: Modifier,
                                )
                                .padding(horizontal = 9.dp, vertical = 7.dp),
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Text("${index + 1}", color = TextSecondary, fontSize = 8.sp, fontFamily = FontFamily.Monospace)
                                Spacer(Modifier.width(5.dp))
                                Text(
                                    activity.activityName ?: activity.topic ?: activity.activityId,
                                    modifier = Modifier.weight(1f),
                                    fontWeight = FontWeight.SemiBold,
                                    fontSize = 10.sp,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                                Text(activity.count.toString(), color = Danger, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace)
                            }
                            Text(
                                "${activity.activityId} · $processLabel",
                                color = TextSecondary,
                                fontFamily = FontFamily.Monospace,
                                fontSize = 8.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    }
                }
            }
        }
    }
}

private fun shortDefinitionLabel(definitionId: String): String {
    val parts = definitionId.split(':', limit = 3)
    val key = parts.getOrNull(0) ?: definitionId
    val version = parts.getOrNull(1)?.toIntOrNull()
    return if (version == null) key else "$key · v$version"
}

@Composable
private fun TimelineChart(
    dashboard: ProcessDashboard,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    modifier: Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(13.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(13.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(dashboard.timelineTitle, fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                    Text("Старт, полное завершение и инциденты", color = TextSecondary, fontSize = 9.sp)
                }
                ChartLegend("Старт", Primary)
                Spacer(Modifier.width(10.dp))
                ChartLegend("Завершено", Healthy)
                Spacer(Modifier.width(10.dp))
                ChartLegend("Инциденты", Danger)
                Spacer(Modifier.width(12.dp))
                Text(
                    if (expanded) "Свернуть ↙" else "Развернуть ↗",
                    color = Primary,
                    fontSize = 10.sp,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.clickable(onClick = onToggleExpanded),
                )
            }
            if (dashboard.timeline.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("За период нет исторических данных", color = TextSecondary, fontSize = 11.sp)
                }
            } else {
                val points = dashboard.timeline
                val maximum = max(
                    1,
                    points.maxOf { max(it.started, max(it.completed, it.incidents)) },
                )
                val gridColor = Border.copy(alpha = 0.75f)
                val startedColor = Primary.copy(alpha = 0.82f)
                val completedColor = Healthy.copy(alpha = 0.82f)
                val incidentColor = Danger
                Canvas(Modifier.fillMaxWidth().weight(1f)) {
                    val plotHeight = size.height - 8.dp.toPx()
                    repeat(4) { index ->
                        val y = plotHeight * index / 3f
                        drawLine(
                            color = gridColor,
                            start = Offset(0f, y),
                            end = Offset(size.width, y),
                            strokeWidth = 1.dp.toPx(),
                        )
                    }
                    val groupWidth = size.width / points.size
                    val barWidth = (groupWidth * 0.25f).coerceAtMost(12.dp.toPx())
                    val incidentPath = Path()
                    points.forEachIndexed { index, point ->
                        val centerX = groupWidth * index + groupWidth / 2f
                        fun valueY(value: Int): Float = plotHeight - plotHeight * value / maximum.toFloat()
                        drawRect(
                            color = startedColor,
                            topLeft = Offset(centerX - barWidth - 1.dp.toPx(), valueY(point.started)),
                            size = Size(barWidth, plotHeight - valueY(point.started)),
                        )
                        drawRect(
                            color = completedColor,
                            topLeft = Offset(centerX + 1.dp.toPx(), valueY(point.completed)),
                            size = Size(barWidth, plotHeight - valueY(point.completed)),
                        )
                        val incidentY = valueY(point.incidents)
                        if (index == 0) incidentPath.moveTo(centerX, incidentY) else incidentPath.lineTo(centerX, incidentY)
                        drawCircle(incidentColor, radius = 2.dp.toPx(), center = Offset(centerX, incidentY))
                    }
                    drawPath(
                        path = incidentPath,
                        color = incidentColor,
                        style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round),
                    )
                }
                val labelStep = max(1, (points.size / 6.0).roundToInt())
                Row(Modifier.fillMaxWidth().height(38.dp), verticalAlignment = Alignment.Top) {
                    points.filterIndexed { index, _ -> index % labelStep == 0 || index == points.lastIndex }
                        .take(7)
                        .forEach { point ->
                            Text(
                                point.label.replace(" ", "\n"),
                                modifier = Modifier.weight(1f),
                                color = TextSecondary,
                                fontSize = if (expanded) 11.sp else 10.sp,
                                lineHeight = 13.sp,
                                maxLines = 2,
                                textAlign = TextAlign.Center,
                            )
                        }
                }
            }
        }
    }
}

@Composable
private fun ChartLegend(label: String, color: Color) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.width(7.dp).height(7.dp).background(color, RoundedCornerShape(2.dp)))
        Spacer(Modifier.width(4.dp))
        Text(label, color = TextSecondary, fontSize = 8.sp)
    }
}

@Composable
private fun IncidentBreakdownCard(
    dashboard: ProcessDashboard,
    onOpenIncidents: (String?) -> Unit,
    modifier: Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(13.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(13.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Типы инцидентов", fontWeight = FontWeight.SemiBold, fontSize = 13.sp, modifier = Modifier.weight(1f))
                Text(
                    "Все →",
                    color = Primary,
                    fontSize = 10.sp,
                    modifier = Modifier.clickable { onOpenIncidents(null) },
                )
            }
            if (dashboard.incidentTypes.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("Инцидентов нет", color = Healthy, fontSize = 11.sp)
                }
            } else {
                val maxCount = dashboard.incidentTypes.maxOf { it.count }.coerceAtLeast(1)
                dashboard.incidentTypes.take(4).forEach { incident ->
                    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                        Row {
                            Text(
                                incidentTypeLabel(incident.type),
                                modifier = Modifier.weight(1f),
                                fontSize = 9.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            Text(incident.count.toString(), color = Danger, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
                        }
                        Box(Modifier.fillMaxWidth().height(5.dp).background(PrimaryMuted, RoundedCornerShape(3.dp))) {
                            Box(
                                Modifier.fillMaxWidth(incident.count / maxCount.toFloat())
                                    .height(5.dp)
                                    .background(Danger, RoundedCornerShape(3.dp)),
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun CriticalProcessesCard(
    dashboard: ProcessDashboard,
    onOpenIncidents: (String?) -> Unit,
    modifier: Modifier,
) {
    val critical = dashboard.definitions.filter { it.incidents > 0 }.sortedByDescending { it.incidents }.take(4)
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(13.dp),
    ) {
        Column(Modifier.fillMaxSize().padding(13.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
            Text("Критические процессы", fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
            Text("Нажмите на строку, чтобы открыть инциденты", color = TextSecondary, fontSize = 9.sp)
            if (critical.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("Все процессы работают штатно", color = Healthy, fontSize = 11.sp)
                }
            } else {
                critical.forEach { definition ->
                    Row(
                        modifier = Modifier.fillMaxWidth().clickable { onOpenIncidents(definition.id) }
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Box(Modifier.width(6.dp).height(6.dp).background(Danger, RoundedCornerShape(3.dp)))
                        Spacer(Modifier.width(7.dp))
                        Column(Modifier.weight(1f)) {
                            Text(definition.displayName, fontSize = 10.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            Text("${definition.instances} заявок", color = TextSecondary, fontSize = 8.sp)
                        }
                        Text(
                            definition.incidents.toString(),
                            color = Danger,
                            fontWeight = FontWeight.Bold,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun HealthIndicator(
    label: String,
    value: String,
    caption: String,
    accent: Color,
    modifier: Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(10.dp),
    ) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 11.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.width(3.dp).height(30.dp).background(accent, RoundedCornerShape(3.dp)))
            Spacer(Modifier.width(8.dp))
            Column(Modifier.weight(1f)) {
                Text(label, color = TextSecondary, fontSize = 8.sp, fontWeight = FontWeight.Bold)
                Text(caption, color = TextSecondary, fontSize = 8.sp, maxLines = 1)
            }
            Text(value, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace, fontSize = 14.sp)
        }
    }
}

private fun formatDuration(durationMillis: Long?): String {
    if (durationMillis == null) return "Нет данных"
    val minutes = durationMillis / 60_000
    val days = minutes / (24 * 60)
    val hours = minutes % (24 * 60) / 60
    val restMinutes = minutes % 60
    return when {
        days > 0 -> "$days д. $hours ч."
        hours > 0 -> "$hours ч. $restMinutes мин."
        else -> "$restMinutes мин."
    }
}

private fun incidentTypeLabel(type: String): String = when (type) {
    "failedJob" -> "Ошибка задачи"
    "failedExternalTask" -> "Ошибка внешней задачи"
    else -> type.ifBlank { "Неизвестный тип" }
}

@Composable
private fun ViewButton(label: String, selected: Boolean, onClick: () -> Unit) {
    if (selected) {
        Button(onClick = onClick) { Text(label) }
    } else {
        OutlinedButton(onClick = onClick) { Text(label) }
    }
}

@Composable
private fun DefinitionsTable(
    definitions: List<ProcessDefinitionSummary>,
    canOpenCockpit: Boolean,
    liveStatistics: Boolean,
    onOpenDefinition: (String) -> Unit,
    onOpenIncidents: (String) -> Unit,
    modifier: Modifier,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(13.dp),
    ) {
        DefinitionRow(isHeader = true, liveStatistics = liveStatistics)
        HorizontalDivider(color = Border)
        Column {
            definitions.forEach { definition ->
                DefinitionRow(
                    definition = definition,
                    canOpenCockpit = canOpenCockpit,
                    liveStatistics = liveStatistics,
                    onOpenDefinition = onOpenDefinition,
                    onOpenIncidents = onOpenIncidents,
                )
                HorizontalDivider(color = Border.copy(alpha = 0.65f))
            }
        }
    }
}

@Composable
private fun DefinitionRow(
    definition: ProcessDefinitionSummary? = null,
    isHeader: Boolean = false,
    canOpenCockpit: Boolean = false,
    liveStatistics: Boolean = true,
    onOpenDefinition: (String) -> Unit = {},
    onOpenIncidents: (String) -> Unit = {},
) {
    ProcessHoverInfo(definition = definition.takeUnless { isHeader }, modifier = Modifier.fillMaxWidth()) { hovered ->
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(
                    when {
                        isHeader -> PrimaryMuted.copy(alpha = 0.48f)
                        hovered -> PrimaryMuted.copy(alpha = 0.62f)
                        else -> Color.Transparent
                    },
                )
                .padding(horizontal = 14.dp, vertical = if (isHeader) 8.dp else 9.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
        if (isHeader) {
            HeaderCell("СОСТОЯНИЕ", 0.8f)
            HeaderCell(if (liveStatistics) "ИНЦИДЕНТЫ" else "СОЗДАНО", 0.7f)
            HeaderCell(if (liveStatistics) "ЗАПУЩЕНО" else "СТАРТОВАЛО", 0.7f)
            HeaderCell("ПРОЦЕСС", 2.1f)
            HeaderCell("ВЕРСИЯ И TTL", 1.0f)
            HeaderCell("РАЗВЁРТЫВАНИЕ", 1.55f)
            HeaderCell("ТЕНАНТ", 0.8f)
            return@Row
        }
        definition ?: return@Row
        TableCell(0.8f) { StateBadge(definition.suspended) }
        TableCell(0.7f) {
            Column(modifier = if (definition.incidents > 0) Modifier.clickable {
                onOpenIncidents(definition.id)
            } else Modifier) {
                Text(
                    definition.incidents.toString(),
                    color = if (definition.incidents > 0) Danger else TextSecondary,
                    fontWeight = if (definition.incidents > 0) FontWeight.Bold else FontWeight.Normal,
                    fontFamily = FontFamily.Monospace,
                )
                if (definition.incidents > 0) Text("Открыть →", color = Danger, fontSize = 8.sp)
            }
        }
        TableCell(0.7f) {
            Text(definition.instances.toString(), fontFamily = FontFamily.Monospace)
        }
        TableCell(2.1f) {
            Column {
                Text(
                    text = definition.displayName,
                    color = if (canOpenCockpit) Primary else TextPrimary,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = if (canOpenCockpit) Modifier.clickable { onOpenDefinition(definition.id) } else Modifier,
                )
                Text(
                    definition.description?.takeIf(String::isNotBlank) ?: definition.key,
                    color = TextSecondary,
                    fontSize = 10.sp,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    fontFamily = if (definition.description.isNullOrBlank()) FontFamily.Monospace else FontFamily.Default,
                )
                if (canOpenCockpit) Text("Открыть процесс →", color = Primary, fontSize = 8.sp)
            }
        }
        TableCell(1.0f) {
            Column {
                Text(
                    "v${definition.version}${definition.versionTag?.let { " · $it" }.orEmpty()}",
                    fontFamily = FontFamily.Monospace,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text("TTL: ${definition.historyTimeToLive?.let { "$it дн." } ?: "—"}", color = TextSecondary, fontSize = 9.sp)
            }
        }
        TableCell(1.55f) {
            Column {
                Text(
                    definition.deploymentId ?: "—",
                    fontFamily = FontFamily.Monospace,
                    fontSize = 10.sp,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    definition.resource ?: definition.category ?: "Ресурс не указан",
                    color = TextSecondary,
                    fontSize = 9.sp,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        TableCell(0.8f) {
            Text(
                definition.tenantId ?: "—",
                color = TextSecondary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
    }
}

@Composable
private fun DefinitionPreviews(
    definitions: List<ProcessDefinitionSummary>,
    canOpenCockpit: Boolean,
    liveStatistics: Boolean,
    onOpenDefinition: (String) -> Unit,
    onOpenIncidents: (String) -> Unit,
    modifier: Modifier,
) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        definitions.chunked(2).forEach { rowDefinitions ->
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                rowDefinitions.forEach { definition ->
                    DefinitionPreviewCard(
                        definition = definition,
                        canOpenCockpit = canOpenCockpit,
                        liveStatistics = liveStatistics,
                        onOpenDefinition = onOpenDefinition,
                        onOpenIncidents = onOpenIncidents,
                        modifier = Modifier.weight(1f),
                    )
                }
                if (rowDefinitions.size == 1) Spacer(Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun DefinitionPreviewCard(
    definition: ProcessDefinitionSummary,
    canOpenCockpit: Boolean,
    liveStatistics: Boolean,
    onOpenDefinition: (String) -> Unit,
    onOpenIncidents: (String) -> Unit,
    modifier: Modifier,
) {
    ProcessHoverInfo(definition = definition, modifier = modifier) { hovered ->
        Card(
            modifier = Modifier.fillMaxWidth().then(
                if (canOpenCockpit) Modifier.clickable { onOpenDefinition(definition.id) } else Modifier,
            ),
            colors = CardDefaults.cardColors(containerColor = if (hovered) PrimaryMuted.copy(alpha = 0.62f) else Surface),
            border = BorderStroke(
                1.dp,
                when {
                    hovered -> Primary.copy(alpha = 0.65f)
                    definition.incidents > 0 -> Danger.copy(alpha = 0.5f)
                    else -> Border
                },
            ),
            shape = RoundedCornerShape(13.dp),
        ) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(9.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                StateBadge(definition.suspended)
                Spacer(Modifier.weight(1f))
                Text(
                    "v${definition.version}${definition.versionTag?.let { " · $it" }.orEmpty()}",
                    color = TextSecondary,
                    fontFamily = FontFamily.Monospace,
                )
            }
            Column {
                Text(definition.displayName, fontWeight = FontWeight.SemiBold, fontSize = 15.sp)
                Text(definition.key, color = TextSecondary, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
                definition.description?.takeIf(String::isNotBlank)?.let {
                    Text(it, color = TextSecondary, fontSize = 10.sp, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(24.dp)) {
                PreviewMetric(
                    if (liveStatistics) "ИНЦИДЕНТЫ" else "СОЗДАНО ИНЦ.",
                    definition.incidents,
                    if (definition.incidents > 0) Danger else Healthy,
                    onClick = if (definition.incidents > 0) ({ onOpenIncidents(definition.id) }) else null,
                )
                PreviewMetric(if (liveStatistics) "ЗАПУЩЕНО" else "СТАРТОВАЛО", definition.instances, Primary)
                Column {
                    Text("ТЕНАНТ", color = TextSecondary, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    Text(definition.tenantId ?: "—", fontSize = 12.sp)
                }
            }
            HorizontalDivider(color = Border.copy(alpha = 0.7f))
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                PreviewDetail("РЕСУРС", definition.resource ?: "—", Modifier.weight(1.4f))
                PreviewDetail("TTL", definition.historyTimeToLive?.let { "$it дн." } ?: "—", Modifier.weight(0.55f))
                PreviewDetail(
                    "TASKLIST",
                    if (definition.startableInTasklist) "Можно запустить" else "Скрыт",
                    Modifier.weight(0.8f),
                )
            }
            PreviewDetail("ID РАЗВЁРТЫВАНИЯ", definition.deploymentId ?: "—", Modifier.fillMaxWidth())
        }
    }
    }
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun ProcessHoverInfo(
    definition: ProcessDefinitionSummary?,
    modifier: Modifier = Modifier,
    content: @Composable (hovered: Boolean) -> Unit,
) {
    var hovered by remember(definition?.id) { mutableStateOf(false) }
    var cursorPosition by remember(definition?.id) { mutableStateOf(Offset.Zero) }
    val hoverModifier = if (definition == null) {
        Modifier
    } else {
        Modifier
            .onPointerEvent(PointerEventType.Enter) { event ->
                hovered = true
                event.changes.firstOrNull()?.position?.let { cursorPosition = it }
            }
            .onPointerEvent(PointerEventType.Move) { event ->
                event.changes.firstOrNull()?.position?.let { cursorPosition = it }
            }
            .onPointerEvent(PointerEventType.Exit) { hovered = false }
    }
    Box(modifier = modifier.then(hoverModifier)) {
        content(hovered)
        if (hovered && definition != null) {
            DisableSelection {
                Popup(
                    alignment = Alignment.TopStart,
                    offset = IntOffset(
                        x = cursorPosition.x.roundToInt() + 16,
                        y = cursorPosition.y.roundToInt() + 16,
                    ),
                    properties = PopupProperties(focusable = false),
                ) {
                    SelectionContainer {
                        ProcessInfoPopup(definition)
                    }
                }
            }
        }
    }
}

@Composable
private fun ProcessInfoPopup(definition: ProcessDefinitionSummary) {
    Card(
        modifier = Modifier.width(330.dp),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Primary.copy(alpha = 0.45f)),
        shape = RoundedCornerShape(12.dp),
    ) {
        Column(Modifier.padding(13.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(definition.displayName, fontWeight = FontWeight.Bold, fontSize = 14.sp)
                    Text(definition.key, color = TextSecondary, fontFamily = FontFamily.Monospace, fontSize = 10.sp)
                }
                StateBadge(definition.suspended)
            }
            definition.description?.takeIf(String::isNotBlank)?.let {
                Text(it, color = TextSecondary, fontSize = 10.sp, maxLines = 3, overflow = TextOverflow.Ellipsis)
            }
            HorizontalDivider(color = Border)
            Row(horizontalArrangement = Arrangement.spacedBy(18.dp)) {
                PopupMetric("ВЕРСИЯ", "v${definition.version}${definition.versionTag?.let { " · $it" }.orEmpty()}", Primary)
                PopupMetric("ЗАЯВКИ", definition.instances.toString(), Healthy)
                PopupMetric("ИНЦИДЕНТЫ", definition.incidents.toString(), if (definition.incidents > 0) Danger else Healthy)
            }
            PopupInfoRow("Deployment", definition.deploymentId ?: "—")
            PopupInfoRow("BPMN-ресурс", definition.resource ?: "—")
            PopupInfoRow("Тенант", definition.tenantId ?: "Без тенанта")
            PopupInfoRow("Хранение истории", definition.historyTimeToLive?.let { "$it дней" } ?: "Не задано")
            Text("Нажмите на название, чтобы открыть процесс; на инциденты — чтобы открыть их список.", color = TextSecondary, fontSize = 9.sp)
        }
    }
}

@Composable
private fun PopupMetric(label: String, value: String, color: Color) {
    Column {
        Text(label, color = TextSecondary, fontSize = 8.sp, fontWeight = FontWeight.Bold)
        Text(value, color = color, fontSize = 12.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace)
    }
}

@Composable
private fun PopupInfoRow(label: String, value: String) {
    Row {
        Text(label, color = TextSecondary, fontSize = 9.sp, modifier = Modifier.width(105.dp))
        Text(value, fontSize = 9.sp, fontFamily = FontFamily.Monospace, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

@Composable
private fun PreviewMetric(label: String, value: Int, color: Color, onClick: (() -> Unit)? = null) {
    Column(modifier = if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier) {
        Text(label, color = TextSecondary, fontSize = 9.sp, fontWeight = FontWeight.Bold)
        Text(value.toString(), color = color, fontSize = 17.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace)
        if (onClick != null) Text("Открыть →", color = Danger, fontSize = 9.sp)
    }
}

@Composable
private fun PreviewDetail(label: String, value: String, modifier: Modifier) {
    Column(modifier) {
        Text(label, color = TextSecondary, fontSize = 8.sp, fontWeight = FontWeight.Bold)
        Text(
            value,
            fontSize = 10.sp,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            fontFamily = FontFamily.Monospace,
        )
    }
}

@Composable
private fun StateBadge(suspended: Boolean) {
    val color = if (suspended) Warning else Healthy
    Box(
        modifier = Modifier.background(color.copy(alpha = 0.12f), RoundedCornerShape(6.dp)).padding(horizontal = 8.dp, vertical = 5.dp),
    ) {
        Text(
            if (suspended) "ПРИОСТАНОВЛЕН" else "АКТИВЕН",
            color = color,
            fontSize = 9.sp,
            fontWeight = FontWeight.Bold,
        )
    }
}

@Composable
private fun RowScope.HeaderCell(text: String, weight: Float) {
    Text(
        text = text,
        modifier = Modifier.weight(weight),
        color = TextSecondary,
        fontSize = 9.sp,
        fontWeight = FontWeight.Bold,
        letterSpacing = 0.7.sp,
    )
}

@Composable
private fun RowScope.TableCell(weight: Float, content: @Composable () -> Unit) {
    Box(Modifier.weight(weight), contentAlignment = Alignment.CenterStart) { content() }
}

@Composable
private fun MetricCard(
    label: String,
    value: String,
    caption: String,
    accent: Color,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(13.dp),
    ) {
        Column(Modifier.padding(horizontal = 13.dp, vertical = 11.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(label, color = TextSecondary, fontSize = 9.sp, fontWeight = FontWeight.Bold, letterSpacing = 0.8.sp)
            Text(value, fontSize = 18.sp, fontWeight = FontWeight.SemiBold, fontFamily = FontFamily.Monospace)
            Text(caption, color = accent, fontSize = 10.sp)
        }
    }
}
