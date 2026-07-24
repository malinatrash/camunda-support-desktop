package com.malinatrash.camundasupport.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.DisableSelection
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.data.TextClipboard
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.ProcessDefinitionSummary
import com.malinatrash.camundasupport.model.ProcessInstanceSummary
import com.malinatrash.camundasupport.model.ProcessVariableCatalog
import com.malinatrash.camundasupport.model.ProcessVariableDescriptor
import com.malinatrash.camundasupport.model.VariableValueType
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
import kotlinx.coroutines.launch

@Composable
fun ProcessesScreen(
    connection: CamundaConnection?,
    camundaApi: CamundaApi,
    textClipboard: TextClipboard,
    onAddConnection: () -> Unit,
    onOpenInstance: (String) -> Unit,
) {
    var definitions by remember { mutableStateOf<List<ProcessDefinitionSummary>>(emptyList()) }
    var definitionsLoading by remember { mutableStateOf(false) }
    var definitionsError by remember { mutableStateOf<String?>(null) }
    var selectedDefinition by remember { mutableStateOf<ProcessDefinitionSummary?>(null) }
    var processMenuOpen by remember { mutableStateOf(false) }
    var processFilter by remember { mutableStateOf("") }

    var variableCatalog by remember { mutableStateOf<ProcessVariableCatalog?>(null) }
    var variablesLoading by remember { mutableStateOf(false) }
    var variablesError by remember { mutableStateOf<String?>(null) }
    var selectedVariable by remember { mutableStateOf<ProcessVariableDescriptor?>(null) }
    var variableMenuOpen by remember { mutableStateOf(false) }
    var variableFilter by remember { mutableStateOf("") }
    var variableCatalogRefreshToken by remember { mutableIntStateOf(0) }

    var variableValue by remember { mutableStateOf("") }
    var valueType by remember { mutableStateOf(VariableValueType.Auto) }
    var typeMenuOpen by remember { mutableStateOf(false) }
    var results by remember { mutableStateOf<List<ProcessInstanceSummary>?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val coroutineScope = rememberCoroutineScope()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(connection?.id) {
        definitions = emptyList()
        selectedDefinition = null
        variableCatalog = null
        selectedVariable = null
        results = null
        definitionsError = null
        if (connection == null) return@LaunchedEffect
        definitionsLoading = true
        when (val loaded = camundaApi.loadSearchProcessDefinitions(connection)) {
            is CamundaApiResult.Success -> definitions = loaded.value
            is CamundaApiResult.Failure -> definitionsError = loaded.message
        }
        definitionsLoading = false
    }

    LaunchedEffect(connection?.id, selectedDefinition?.key, variableCatalogRefreshToken) {
        variableCatalog = null
        selectedVariable = null
        variableFilter = ""
        variablesError = null
        results = null
        val currentConnection = connection ?: return@LaunchedEffect
        val definition = selectedDefinition ?: return@LaunchedEffect
        variablesLoading = true
        when (val loaded = camundaApi.loadProcessVariableCatalog(currentConnection, definition.key)) {
            is CamundaApiResult.Success -> variableCatalog = loaded.value
            is CamundaApiResult.Failure -> variablesError = loaded.message
        }
        variablesLoading = false
    }

    Box(Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            SectionTitle(
                title = "Поиск заявок",
                description = "Выберите процесс и реальную переменную его активных заявок — вводить ключ вручную больше не нужно.",
            )

            if (connection == null) {
                EmptyPanel(
                    title = "Требуется подключение",
                    description = "Выберите или добавьте подключение Camunda перед поиском заявок.",
                    actionLabel = "Добавить подключение",
                    onAction = onAddConnection,
                )
                return@Column
            }

            SearchForm(
                connection = connection,
                definitions = definitions,
                definitionsLoading = definitionsLoading,
                definitionsError = definitionsError,
                selectedDefinition = selectedDefinition,
                processMenuOpen = processMenuOpen,
                processFilter = processFilter,
                onProcessMenuOpen = { processMenuOpen = it },
                onProcessFilterChange = { processFilter = it },
                onDefinitionSelected = {
                    selectedDefinition = it
                    processMenuOpen = false
                    processFilter = ""
                    error = null
                },
                catalog = variableCatalog,
                variablesLoading = variablesLoading,
                variablesError = variablesError,
                selectedVariable = selectedVariable,
                variableMenuOpen = variableMenuOpen,
                variableFilter = variableFilter,
                onVariableMenuOpen = { variableMenuOpen = it },
                onVariableFilterChange = { variableFilter = it },
                onVariableSelected = {
                    selectedVariable = it
                    variableMenuOpen = false
                    variableFilter = ""
                    error = null
                },
                onRefreshVariables = {
                    selectedDefinition?.let { definition ->
                        camundaApi.invalidateProcessVariableCatalog(connection, definition.key)
                        variableCatalogRefreshToken += 1
                    }
                },
                variableValue = variableValue,
                onVariableValueChange = {
                    variableValue = it
                    error = null
                },
                valueType = valueType,
                typeMenuOpen = typeMenuOpen,
                onTypeMenuOpen = { typeMenuOpen = it },
                onValueTypeSelected = {
                    valueType = it
                    typeMenuOpen = false
                },
                isLoading = isLoading,
                onSearch = {
                    val definition = selectedDefinition
                    val variable = selectedVariable
                    when {
                        definition == null -> error = "Сначала выберите процесс."
                        variable == null -> error = "Выберите переменную из списка процесса."
                        variableValue.isBlank() -> error = "Укажите точное значение переменной."
                        else -> {
                            isLoading = true
                            error = null
                            coroutineScope.launch {
                                when (
                                    val loaded = camundaApi.searchProcessInstances(
                                        connection = connection,
                                        processDefinitionKey = definition.key,
                                        variableName = variable.name,
                                        variableValue = variableValue,
                                        valueType = valueType,
                                    )
                                ) {
                                    is CamundaApiResult.Success -> results = loaded.value
                                    is CamundaApiResult.Failure -> error = loaded.message
                                }
                                isLoading = false
                            }
                        }
                    }
                },
            )

            if (isLoading) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    CircularProgressIndicator(Modifier.width(18.dp).height(18.dp), strokeWidth = 2.dp)
                    Text("Ищем заявки выбранного процесса…", color = TextSecondary)
                }
            }

            error?.let { message ->
                Text(
                    text = message,
                    color = Danger,
                    fontSize = 12.sp,
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(Danger.copy(alpha = 0.08f), RoundedCornerShape(9.dp))
                        .padding(12.dp),
                )
            }

            results?.let { instances ->
                Column {
                    Text("Найдено активных заявок: ${instances.size}", fontWeight = FontWeight.SemiBold, fontSize = 16.sp)
                    Text(
                        "${selectedDefinition?.displayName ?: "Процесс"} · " +
                            "${selectedVariable?.name} = $variableValue · тип: ${valueType.label}",
                        color = TextSecondary,
                        fontSize = 11.sp,
                    )
                }
                if (instances.isEmpty()) {
                    EmptyPanel(
                        title = "Заявки не найдены",
                        description = "Для выбранного процесса нет активных заявок с таким точным значением.",
                    )
                } else {
                    InstancesTable(
                        instances = instances,
                        onOpenInstance = onOpenInstance,
                        onCopyBusinessKey = { businessKey ->
                            val message = if (textClipboard.copy(businessKey)) {
                                "Бизнес-ключ $businessKey скопирован"
                            } else {
                                "Не удалось скопировать бизнес-ключ"
                            }
                            coroutineScope.launch { snackbarHostState.showSnackbar(message) }
                        },
                        modifier = Modifier.weight(1f),
                    )
                }
            } ?: run {
                if (!isLoading && error == null) {
                    EmptyPanel(
                        title = "Начните с выбора процесса",
                        description = "После выбора приложение прочитает переменные его активных заявок и покажет конкретный список ключей.",
                    )
                }
            }
        }

        SnackbarHost(
            hostState = snackbarHostState,
            modifier = Modifier.align(Alignment.BottomCenter).padding(16.dp),
        )
    }
}

@Composable
private fun SearchForm(
    connection: CamundaConnection,
    definitions: List<ProcessDefinitionSummary>,
    definitionsLoading: Boolean,
    definitionsError: String?,
    selectedDefinition: ProcessDefinitionSummary?,
    processMenuOpen: Boolean,
    processFilter: String,
    onProcessMenuOpen: (Boolean) -> Unit,
    onProcessFilterChange: (String) -> Unit,
    onDefinitionSelected: (ProcessDefinitionSummary) -> Unit,
    catalog: ProcessVariableCatalog?,
    variablesLoading: Boolean,
    variablesError: String?,
    selectedVariable: ProcessVariableDescriptor?,
    variableMenuOpen: Boolean,
    variableFilter: String,
    onVariableMenuOpen: (Boolean) -> Unit,
    onVariableFilterChange: (String) -> Unit,
    onVariableSelected: (ProcessVariableDescriptor) -> Unit,
    onRefreshVariables: () -> Unit,
    variableValue: String,
    onVariableValueChange: (String) -> Unit,
    valueType: VariableValueType,
    typeMenuOpen: Boolean,
    onTypeMenuOpen: (Boolean) -> Unit,
    onValueTypeSelected: (VariableValueType) -> Unit,
    isLoading: Boolean,
    onSearch: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(10.dp),
    ) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                SearchProcessPicker(
                    definitions = definitions,
                    loading = definitionsLoading,
                    error = definitionsError,
                    selected = selectedDefinition,
                    menuOpen = processMenuOpen,
                    filter = processFilter,
                    onMenuOpen = onProcessMenuOpen,
                    onFilterChange = onProcessFilterChange,
                    onSelected = onDefinitionSelected,
                    modifier = Modifier.weight(1.15f),
                )
                SearchVariablePicker(
                    catalog = catalog,
                    loading = variablesLoading,
                    error = variablesError,
                    selected = selectedVariable,
                    enabled = selectedDefinition != null,
                    menuOpen = variableMenuOpen,
                    filter = variableFilter,
                    onMenuOpen = onVariableMenuOpen,
                    onFilterChange = onVariableFilterChange,
                    onSelected = onVariableSelected,
                    modifier = Modifier.weight(1.15f),
                )
                OutlinedTextField(
                    value = variableValue,
                    onValueChange = onVariableValueChange,
                    label = { Text("Точное значение") },
                    placeholder = { Text("ID заявки или другое значение") },
                    singleLine = true,
                    modifier = Modifier.weight(1f),
                )
                Box {
                    OutlinedButton(onClick = { onTypeMenuOpen(true) }) {
                        Text("Тип: ${valueType.label} ⌄")
                    }
                    DisableSelection {
                        DropdownMenu(
                            expanded = typeMenuOpen,
                            onDismissRequest = { onTypeMenuOpen(false) },
                        ) {
                            VariableValueType.entries.forEach { type ->
                                DropdownMenuItem(
                                    text = { Text(type.label) },
                                    onClick = { onValueTypeSelected(type) },
                                )
                            }
                        }
                    }
                }
                Button(enabled = !isLoading, onClick = onSearch) {
                    Text(if (isLoading) "Ищем…" else "Найти")
                }
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    when {
                        selectedDefinition == null -> "1. Выберите процесс"
                        variablesLoading -> "2. Читаем переменные активных заявок…"
                        catalog != null -> "2. Найдено ключей: ${catalog.variables.size} " +
                            "в ${catalog.inspectedInstanceCount} активных заявках · кэш между сессиями до 24 ч."
                        else -> "2. Выберите переменную"
                    },
                    color = TextSecondary,
                    fontSize = 11.sp,
                )
                if (catalog?.instancesTruncated == true) {
                    Text(
                        " · список построен по первым 100 активным заявкам",
                        color = Warning,
                        fontSize = 11.sp,
                    )
                }
                Spacer(Modifier.width(10.dp))
                Text(
                    text = "Обновить ключи",
                    color = if (selectedDefinition != null && !variablesLoading) Primary else TextSecondary,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.clickable(
                        enabled = selectedDefinition != null && !variablesLoading,
                        onClick = onRefreshVariables,
                    ),
                )
                Spacer(Modifier.weight(1f))
                Text(
                    "${connection.name} · POST /process-instance · точное совпадение",
                    color = TextSecondary,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 10.sp,
                )
            }
        }
    }
}

@Composable
private fun SearchProcessPicker(
    definitions: List<ProcessDefinitionSummary>,
    loading: Boolean,
    error: String?,
    selected: ProcessDefinitionSummary?,
    menuOpen: Boolean,
    filter: String,
    onMenuOpen: (Boolean) -> Unit,
    onFilterChange: (String) -> Unit,
    onSelected: (ProcessDefinitionSummary) -> Unit,
    modifier: Modifier,
) {
    val filtered = definitions.filter {
        filter.isBlank() ||
            it.displayName.contains(filter, ignoreCase = true) ||
            it.key.contains(filter, ignoreCase = true)
    }
    Box(modifier) {
        OutlinedButton(
            onClick = { onMenuOpen(true) },
            enabled = !loading && error == null && definitions.isNotEmpty(),
            modifier = Modifier.fillMaxWidth().height(56.dp),
        ) {
            Column(Modifier.weight(1f), horizontalAlignment = Alignment.Start) {
                Text("ПРОЦЕСС", color = TextSecondary, fontSize = 8.sp, fontWeight = FontWeight.Bold)
                Text(
                    when {
                        loading -> "Загружаем процессы…"
                        error != null -> "Не удалось загрузить"
                        selected != null -> selected.displayName
                        else -> "Выберите процесс"
                    },
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                selected?.let { Text(it.key, color = TextSecondary, fontSize = 9.sp, fontFamily = FontFamily.Monospace) }
            }
            Text("⌄")
        }
        DisableSelection {
            DropdownMenu(
                expanded = menuOpen,
                onDismissRequest = { onMenuOpen(false) },
                modifier = Modifier.width(440.dp).heightIn(max = 520.dp),
            ) {
                OutlinedTextField(
                    value = filter,
                    onValueChange = onFilterChange,
                    placeholder = { Text("Фильтр по названию или ключу") },
                    singleLine = true,
                    modifier = Modifier.width(420.dp).padding(horizontal = 8.dp, vertical = 6.dp),
                )
                Text("Найдено: ${filtered.size}", color = TextSecondary, fontSize = 10.sp, modifier = Modifier.padding(10.dp))
                filtered.forEach { definition ->
                    DropdownMenuItem(
                        text = {
                            Column {
                                Text(definition.displayName, fontWeight = FontWeight.Medium)
                                Text(
                                    "${definition.key} · v${definition.version}",
                                    color = TextSecondary,
                                    fontSize = 9.sp,
                                    fontFamily = FontFamily.Monospace,
                                )
                            }
                        },
                        onClick = { onSelected(definition) },
                    )
                }
            }
        }
    }
}

@Composable
private fun SearchVariablePicker(
    catalog: ProcessVariableCatalog?,
    loading: Boolean,
    error: String?,
    selected: ProcessVariableDescriptor?,
    enabled: Boolean,
    menuOpen: Boolean,
    filter: String,
    onMenuOpen: (Boolean) -> Unit,
    onFilterChange: (String) -> Unit,
    onSelected: (ProcessVariableDescriptor) -> Unit,
    modifier: Modifier,
) {
    val variables = catalog?.variables.orEmpty()
    val filtered = variables.filter { filter.isBlank() || it.name.contains(filter, ignoreCase = true) }
    Box(modifier) {
        OutlinedButton(
            onClick = { onMenuOpen(true) },
            enabled = enabled && !loading && error == null && variables.isNotEmpty(),
            modifier = Modifier.fillMaxWidth().height(56.dp),
        ) {
            Column(Modifier.weight(1f), horizontalAlignment = Alignment.Start) {
                Text("ПЕРЕМЕННАЯ", color = TextSecondary, fontSize = 8.sp, fontWeight = FontWeight.Bold)
                Text(
                    when {
                        !enabled -> "Сначала выберите процесс"
                        loading -> "Читаем ключи…"
                        error != null -> "Не удалось загрузить"
                        variables.isEmpty() && catalog != null -> "У активных заявок нет переменных"
                        selected != null -> selected.name
                        else -> "Выберите ключ из списка"
                    },
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    fontFamily = if (selected != null) FontFamily.Monospace else FontFamily.Default,
                )
                selected?.let {
                    Text(it.types.joinToString(), color = TextSecondary, fontSize = 9.sp, maxLines = 1)
                }
            }
            Text("⌄")
        }
        DisableSelection {
            DropdownMenu(
                expanded = menuOpen,
                onDismissRequest = { onMenuOpen(false) },
                modifier = Modifier.width(440.dp).heightIn(max = 520.dp),
            ) {
                OutlinedTextField(
                    value = filter,
                    onValueChange = onFilterChange,
                    placeholder = { Text("Точный фильтр по названию ключа") },
                    singleLine = true,
                    modifier = Modifier.width(420.dp).padding(horizontal = 8.dp, vertical = 6.dp),
                )
                Text("Найдено: ${filtered.size}", color = TextSecondary, fontSize = 10.sp, modifier = Modifier.padding(10.dp))
                filtered.forEach { variable ->
                    DropdownMenuItem(
                        text = {
                            Column {
                                Text(variable.name, fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium)
                                Text(
                                    "${variable.types.joinToString()} · встречается ${variable.occurrences} раз",
                                    color = TextSecondary,
                                    fontSize = 9.sp,
                                )
                            }
                        },
                        onClick = { onSelected(variable) },
                    )
                }
            }
        }
    }
}

@Composable
private fun InstancesTable(
    instances: List<ProcessInstanceSummary>,
    onOpenInstance: (String) -> Unit,
    onCopyBusinessKey: (String) -> Unit,
    modifier: Modifier,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(13.dp),
    ) {
        InstanceHeader()
        HorizontalDivider(color = Border)
        LazyColumn {
            items(instances, key = { it.id }) { instance ->
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { onOpenInstance(instance.id) }
                        .padding(horizontal = 16.dp, vertical = 13.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    InstanceCell(0.75f) {
                        val color = if (instance.suspended) Warning else Healthy
                        Text(
                            if (instance.suspended) "ПРИОСТАНОВЛЕН" else "ЗАПУЩЕН",
                            color = color,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                        )
                    }
                    InstanceCell(1.4f) {
                        val businessKey = instance.businessKey
                        Text(
                            businessKey ?: "—",
                            color = if (businessKey == null) TextSecondary else Primary,
                            fontWeight = if (businessKey == null) FontWeight.Normal else FontWeight.SemiBold,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = if (businessKey == null) Modifier else Modifier.clickable {
                                onCopyBusinessKey(businessKey)
                            },
                        )
                    }
                    InstanceCell(1.35f) {
                        Column {
                            Text(instance.definitionKey, fontWeight = FontWeight.Medium)
                            Text(instance.definitionId, color = TextSecondary, fontSize = 9.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    InstanceCell(1.5f) {
                        Text(instance.id, color = TextSecondary, fontFamily = FontFamily.Monospace, fontSize = 10.sp)
                    }
                    InstanceCell(0.8f) {
                        Text(instance.tenantId ?: "—", color = TextSecondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                    InstanceCell(0.65f) {
                        Text("Открыть →", color = Primary, fontWeight = FontWeight.SemiBold)
                    }
                }
                HorizontalDivider(color = Border.copy(alpha = 0.65f))
            }
        }
    }
}

@Composable
private fun InstanceHeader() {
    Row(
        modifier = Modifier.fillMaxWidth().background(PrimaryMuted.copy(alpha = 0.48f)).padding(horizontal = 16.dp, vertical = 11.dp),
    ) {
        InstanceHeaderCell("СОСТОЯНИЕ", 0.75f)
        InstanceHeaderCell("БИЗНЕС-КЛЮЧ · КЛИК = КОПИЯ", 1.4f)
        InstanceHeaderCell("ПРОЦЕСС", 1.35f)
        InstanceHeaderCell("ID ЭКЗЕМПЛЯРА", 1.5f)
        InstanceHeaderCell("ТЕНАНТ", 0.8f)
        InstanceHeaderCell("ДЕЙСТВИЕ", 0.65f)
    }
}

@Composable
private fun RowScope.InstanceHeaderCell(text: String, weight: Float) {
    Text(
        text,
        modifier = Modifier.weight(weight),
        color = TextSecondary,
        fontSize = 9.sp,
        fontWeight = FontWeight.Bold,
        letterSpacing = 0.7.sp,
    )
}

@Composable
private fun RowScope.InstanceCell(weight: Float, content: @Composable () -> Unit) {
    Box(Modifier.weight(weight), contentAlignment = Alignment.CenterStart) { content() }
}
