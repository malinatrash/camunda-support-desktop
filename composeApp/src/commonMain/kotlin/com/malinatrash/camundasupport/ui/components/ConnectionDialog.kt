package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.DialogWindow
import androidx.compose.ui.window.rememberDialogState
import kotlinx.coroutines.launch
import com.malinatrash.camundasupport.data.ConnectionTester
import com.malinatrash.camundasupport.data.ConnectionTestResult
import com.malinatrash.camundasupport.model.ConnectionDraft
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.model.deriveCockpitUrl
import com.malinatrash.camundasupport.model.validateConnectionDraft
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.SurfaceElevated
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun ConnectionDialog(
    connectionTester: ConnectionTester,
    onDismiss: () -> Unit,
    onSave: (ConnectionDraft, String?) -> Unit,
) {
    var draft by remember { mutableStateOf(ConnectionDraft()) }
    var submitted by remember { mutableStateOf(false) }
    var isChecking by remember { mutableStateOf(false) }
    var pingError by remember { mutableStateOf<String?>(null) }
    var cockpitUrlOverridden by remember { mutableStateOf(false) }
    val coroutineScope = rememberCoroutineScope()
    val validation = validateConnectionDraft(draft)

    DialogWindow(
        onCloseRequest = onDismiss,
        title = "Добавление подключения Camunda",
        state = rememberDialogState(width = 600.dp, height = 660.dp),
        resizable = true,
    ) {
        SelectionContainer {
            Card(
                modifier = Modifier.fillMaxSize().padding(12.dp),
                shape = RoundedCornerShape(12.dp),
                border = BorderStroke(1.dp, Border),
                colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
            ) {
                Column(Modifier.fillMaxSize().padding(20.dp)) {
                Text("Новое подключение", fontSize = 20.sp, fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.height(6.dp))
                Text(
                    "Настройки подключения хранятся только на этом компьютере.",
                    color = TextSecondary,
                    fontSize = 12.sp,
                )
                Spacer(Modifier.height(16.dp))

                Column(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .verticalScroll(rememberScrollState()),
                ) {
                    OutlinedTextField(
                        value = draft.name,
                        onValueChange = { draft = draft.copy(name = it) },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Название подключения") },
                        placeholder = { Text("B2C Продакшен") },
                        singleLine = true,
                        isError = submitted && validation.nameError != null,
                        supportingText = submittedError(if (submitted) validation.nameError else null),
                    )
                    Spacer(Modifier.height(10.dp))
                    OutlinedTextField(
                        value = draft.restUrl,
                        onValueChange = {
                            draft = draft.copy(
                                restUrl = it,
                                cockpitUrl = if (cockpitUrlOverridden) draft.cockpitUrl else deriveCockpitUrl(it),
                            )
                            pingError = null
                        },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Адрес Camunda REST") },
                        placeholder = { Text("https://camunda.example.kz/engine-rest") },
                        singleLine = true,
                        isError = submitted && validation.restUrlError != null,
                        supportingText = submittedError(if (submitted) validation.restUrlError else null),
                    )
                    Spacer(Modifier.height(10.dp))
                    OutlinedTextField(
                        value = draft.cockpitUrl,
                        onValueChange = {
                            cockpitUrlOverridden = true
                            draft = draft.copy(cockpitUrl = it)
                        },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Адрес Cockpit — создан автоматически") },
                        placeholder = { Text("https://camunda.example.kz/camunda/app/cockpit") },
                        singleLine = true,
                        isError = submitted && validation.cockpitUrlError != null,
                        supportingText = submittedError(if (submitted) validation.cockpitUrlError else null),
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            "Создаётся из домена REST. Изменяйте только при нестандартном адресе Cockpit.",
                            color = TextSecondary,
                            fontSize = 10.sp,
                            modifier = Modifier.weight(1f),
                        )
                        if (cockpitUrlOverridden) {
                            OutlinedButton(
                                onClick = {
                                    cockpitUrlOverridden = false
                                    draft = draft.copy(cockpitUrl = deriveCockpitUrl(draft.restUrl))
                                },
                            ) {
                                Text("Вернуть авто")
                            }
                        }
                    }

                    Spacer(Modifier.height(16.dp))
                    Text("ОКРУЖЕНИЕ", color = TextSecondary, fontSize = 10.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.sp)
                    Spacer(Modifier.height(8.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Environment.entries.forEach { environment ->
                            EnvironmentOption(
                                environment = environment,
                                selected = draft.environment == environment,
                                onClick = { draft = draft.copy(environment = environment) },
                            )
                        }
                    }
                    Spacer(Modifier.height(8.dp))
                }

                if (isChecking) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(top = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        CircularProgressIndicator(
                            modifier = Modifier.width(18.dp).height(18.dp),
                            color = Primary,
                            strokeWidth = 2.dp,
                        )
                        Text(
                            "Проверяем /engine-rest/version…",
                            color = TextSecondary,
                            fontSize = 12.sp,
                        )
                    }
                }

                pingError?.let { message ->
                    Text(
                        text = message,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(top = 14.dp)
                            .background(Danger.copy(alpha = 0.08f), RoundedCornerShape(8.dp))
                            .padding(12.dp),
                        color = Danger,
                        fontSize = 12.sp,
                    )
                }

                Spacer(Modifier.height(16.dp))
                HorizontalDivider(color = Border)
                Spacer(Modifier.height(16.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedButton(
                        onClick = onDismiss,
                        enabled = !isChecking,
                    ) {
                        Text("Отмена")
                    }
                    Spacer(Modifier.width(10.dp))
                    Button(
                        enabled = !isChecking,
                        onClick = {
                            submitted = true
                            pingError = null
                            if (!validation.isValid) return@Button

                            isChecking = true
                            coroutineScope.launch {
                                when (val result = connectionTester.test(draft.restUrl)) {
                                    is ConnectionTestResult.Success -> onSave(draft, result.engineVersion)
                                    is ConnectionTestResult.Failure -> {
                                        pingError = result.message
                                        isChecking = false
                                    }
                                }
                            }
                        },
                    ) {
                        Text(if (isChecking) "Проверяем…" else "Проверить и добавить")
                    }
                }
                }
            }
        }
    }
}

@Composable
private fun EnvironmentOption(
    environment: Environment,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val color = environmentColor(environment)
    Box(
        modifier = Modifier
            .background(
                if (selected) color.copy(alpha = 0.16f) else Color.Transparent,
                RoundedCornerShape(8.dp),
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 8.dp),
    ) {
        Text(
            text = environment.shortLabel,
            color = if (selected) color else TextSecondary,
            fontSize = 10.sp,
            fontWeight = FontWeight.Bold,
        )
    }
}

private fun submittedError(error: String?): (@Composable () -> Unit)? = error?.let { message ->
    { Text(message) }
}
