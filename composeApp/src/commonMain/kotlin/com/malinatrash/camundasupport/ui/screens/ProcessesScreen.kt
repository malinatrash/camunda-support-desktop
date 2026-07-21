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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.text.selection.DisableSelection
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import kotlinx.coroutines.launch
import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.data.VariableKeyRepository
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.ProcessInstanceSummary
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

@Composable
fun ProcessesScreen(
    connection: CamundaConnection?,
    camundaApi: CamundaApi,
    variableKeyRepository: VariableKeyRepository,
    onAddConnection: () -> Unit,
    onOpenInstance: (String) -> Unit,
) {
    var variableName by remember { mutableStateOf("") }
    var variableValue by remember { mutableStateOf("") }
    var valueType by remember { mutableStateOf(VariableValueType.Auto) }
    var rememberKey by remember { mutableStateOf(true) }
    var savedKeys by remember { mutableStateOf(emptyList<String>()) }
    var keyMenuOpen by remember { mutableStateOf(false) }
    var typeMenuOpen by remember { mutableStateOf(false) }
    var results by remember { mutableStateOf<List<ProcessInstanceSummary>?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val coroutineScope = rememberCoroutineScope()

    LaunchedEffect(connection?.id) {
        savedKeys = connection?.let { variableKeyRepository.load(it.id) }.orEmpty()
        results = null
        error = null
    }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        SectionTitle(
            title = "Поиск экземпляров процесса",
            description = "Поиск запущенных экземпляров по любой переменной с точным совпадением значения.",
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

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Surface),
            border = BorderStroke(1.dp, Border),
            shape = RoundedCornerShape(10.dp),
        ) {
            Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(9.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Box(Modifier.width(300.dp)) {
                        OutlinedTextField(
                            value = variableName,
                            onValueChange = {
                                variableName = it
                                error = null
                            },
                            label = { Text("Ключ переменной") },
                            placeholder = { Text("applicationId") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                            trailingIcon = {
                                Text(
                                    "⌄",
                                    color = if (savedKeys.isEmpty()) TextSecondary.copy(alpha = 0.45f) else Primary,
                                    fontSize = 18.sp,
                                    modifier = Modifier.clickable(enabled = savedKeys.isNotEmpty()) { keyMenuOpen = true },
                                )
                            },
                        )
                        DisableSelection {
                            DropdownMenu(
                                expanded = keyMenuOpen,
                                onDismissRequest = { keyMenuOpen = false },
                                modifier = Modifier.width(300.dp),
                            ) {
                                SelectionContainer {
                                    Column {
                                        savedKeys.forEach { key ->
                                            DropdownMenuItem(
                                                text = { Text(key, fontFamily = FontFamily.Monospace) },
                                                onClick = {
                                                    variableName = key
                                                    keyMenuOpen = false
                                                },
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    OutlinedTextField(
                        value = variableValue,
                        onValueChange = {
                            variableValue = it
                            error = null
                        },
                        label = { Text("Точное значение") },
                        placeholder = { Text("ID заявки или другое значение") },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                    )

                    Box {
                        OutlinedButton(onClick = { typeMenuOpen = true }) {
                            Text("Тип: ${valueType.label} ⌄")
                        }
                        DisableSelection {
                            DropdownMenu(
                                expanded = typeMenuOpen,
                                onDismissRequest = { typeMenuOpen = false },
                            ) {
                                SelectionContainer {
                                    Column {
                                        VariableValueType.entries.forEach { type ->
                                            DropdownMenuItem(
                                                text = { Text(type.label) },
                                                onClick = {
                                                    valueType = type
                                                    typeMenuOpen = false
                                                },
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Button(
                        enabled = !isLoading,
                        onClick = {
                            val key = variableName.trim()
                            if (key.isEmpty()) {
                                error = "Укажите ключ переменной."
                                return@Button
                            }
                            if (variableValue.isBlank()) {
                                error = "Укажите значение переменной."
                                return@Button
                            }
                            if (rememberKey) {
                                variableKeyRepository.save(connection.id, key)
                                savedKeys = variableKeyRepository.load(connection.id)
                            }
                            isLoading = true
                            error = null
                            coroutineScope.launch {
                                when (
                                    val result = camundaApi.searchProcessInstances(
                                        connection = connection,
                                        variableName = key,
                                        variableValue = variableValue,
                                        valueType = valueType,
                                    )
                                ) {
                                    is CamundaApiResult.Success -> results = result.value
                                    is CamundaApiResult.Failure -> error = result.message
                                }
                                isLoading = false
                            }
                        },
                    ) {
                        Text(if (isLoading) "Ищем…" else "Найти")
                    }
                }

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(checked = rememberKey, onCheckedChange = { rememberKey = it })
                    Text("Запомнить ключ для подключения «${connection.name}»", color = TextSecondary, fontSize = 12.sp)
                    Spacer(Modifier.weight(1f))
                    Text(
                        "POST /process-instance · точное совпадение · «Авто» проверяет примитивные типы",
                        color = TextSecondary,
                        fontFamily = FontFamily.Monospace,
                        fontSize = 10.sp,
                    )
                }
            }
        }

        if (isLoading) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                CircularProgressIndicator(Modifier.width(18.dp).height(18.dp), strokeWidth = 2.dp)
                Text("Ищем запущенные экземпляры процессов…", color = TextSecondary)
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
            Row(verticalAlignment = Alignment.Bottom) {
                Column(Modifier.weight(1f)) {
                    Text("Найдено запущенных экземпляров: ${instances.size}", fontWeight = FontWeight.SemiBold, fontSize = 16.sp)
                    Text("Точное совпадение: $variableName = $variableValue · тип: ${valueType.label}", color = TextSecondary, fontSize = 11.sp)
                }
            }
            if (instances.isEmpty()) {
                EmptyPanel(
                    title = "Запущенные экземпляры не найдены",
                    description = "Проверьте ключ и значение переменной. Завершённые экземпляры не входят в этот поиск.",
                )
            } else {
                InstancesTable(
                    instances = instances,
                    canOpenCockpit = true,
                    onOpenInstance = onOpenInstance,
                    modifier = Modifier.weight(1f),
                )
            }
        } ?: run {
            if (!isLoading && error == null) {
                EmptyPanel(
                    title = "Поиск по переменной процесса",
                    description = "Выберите сохранённый ключ или введите новый. Ключи хранятся локально отдельно для каждого подключения.",
                )
            }
        }
    }
}

@Composable
private fun InstancesTable(
    instances: List<ProcessInstanceSummary>,
    canOpenCockpit: Boolean,
    onOpenInstance: (String) -> Unit,
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
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 13.dp),
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
                        Text(instance.businessKey ?: "—", maxLines = 1, overflow = TextOverflow.Ellipsis)
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
                        Text(
                            "Подробнее",
                            color = if (canOpenCockpit) Primary else TextSecondary.copy(alpha = 0.45f),
                            fontWeight = FontWeight.SemiBold,
                            modifier = if (canOpenCockpit) Modifier.clickable { onOpenInstance(instance.id) } else Modifier,
                        )
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
        InstanceHeaderCell("БИЗНЕС-КЛЮЧ", 1.4f)
        InstanceHeaderCell("ПРОЦЕСС", 1.35f)
        InstanceHeaderCell("ID ЭКЗЕМПЛЯРА", 1.5f)
        InstanceHeaderCell("ТЕНАНТ", 0.8f)
        InstanceHeaderCell("CAMUNDA", 0.65f)
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
