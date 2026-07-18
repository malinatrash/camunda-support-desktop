package com.malinatrash.camundasupport.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.ui.components.EmptyPanel
import com.malinatrash.camundasupport.ui.components.EnvironmentBadge
import com.malinatrash.camundasupport.ui.components.SectionTitle
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun ConnectionsScreen(
    connections: List<CamundaConnection>,
    selectedConnectionId: String?,
    onSelectConnection: (String) -> Unit,
    onAddConnection: () -> Unit,
    onDeleteConnection: (String) -> Unit,
) {
    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.Bottom,
        ) {
            SectionTitle(
                title = "Подключения",
                description = "Окружения Camunda REST, доступные с этого компьютера.",
                modifier = Modifier.weight(1f),
            )
            Button(onClick = onAddConnection) { Text("Добавить подключение") }
        }
        Spacer(Modifier.height(12.dp))

        if (connections.isEmpty()) {
            EmptyPanel(
                title = "Подключения ещё не настроены",
                description = "Настройки подключений хранятся локально; данные авторизации не записываются в профиль подключения.",
                actionLabel = "Добавить подключение",
                onAction = onAddConnection,
            )
            return@Column
        }

        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            connections.forEach { connection ->
                ConnectionCard(
                    connection = connection,
                    selected = connection.id == selectedConnectionId,
                    onClick = { onSelectConnection(connection.id) },
                    onDelete = { onDeleteConnection(connection.id) },
                )
            }
        }
    }
}

@Composable
private fun ConnectionCard(
    connection: CamundaConnection,
    selected: Boolean,
    onClick: () -> Unit,
    onDelete: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(if (selected) 2.dp else 1.dp, if (selected) environmentColorForBorder(connection) else Border),
        shape = RoundedCornerShape(13.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(9.dp)) {
                    Text(connection.name, fontWeight = FontWeight.SemiBold)
                    EnvironmentBadge(connection.environment)
                }
                Text(
                    connection.restUrl,
                    color = TextSecondary,
                    fontSize = 11.sp,
                    fontFamily = FontFamily.Monospace,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            OutlinedButton(onClick = onDelete) { Text("Удалить") }
        }
    }
}

private fun environmentColorForBorder(connection: CamundaConnection) =
    com.malinatrash.camundasupport.ui.components.environmentColor(connection.environment)
