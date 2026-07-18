package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.model.AppDestination
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.PrimaryMuted
import com.malinatrash.camundasupport.ui.theme.SidebarBackground
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun AppSidebar(
    connections: List<CamundaConnection>,
    selectedConnectionId: String?,
    destination: AppDestination,
    onSelectConnection: (String) -> Unit,
    onNavigate: (AppDestination) -> Unit,
    onAddConnection: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .width(232.dp)
            .fillMaxHeight()
            .background(SidebarBackground)
            .padding(horizontal = 12.dp, vertical = 13.dp),
    ) {
        Brand()
        Spacer(Modifier.height(18.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "ПОДКЛЮЧЕНИЯ",
                color = TextSecondary,
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                letterSpacing = 1.1.sp,
                modifier = Modifier.weight(1f),
            )
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(6.dp))
                    .clickable(onClick = onAddConnection)
                    .padding(horizontal = 8.dp, vertical = 3.dp),
            ) {
                Text("+ ДОБАВИТЬ", color = Primary, fontSize = 10.sp, fontWeight = FontWeight.Bold)
            }
        }

        Spacer(Modifier.height(10.dp))
        if (connections.isEmpty()) {
            Text(
                text = "Окружения Camunda ещё не добавлены",
                color = TextSecondary,
                fontSize = 12.sp,
                modifier = Modifier.padding(horizontal = 10.dp, vertical = 12.dp),
            )
        } else {
            connections.forEach { connection ->
                ConnectionItem(
                    connection = connection,
                    selected = connection.id == selectedConnectionId,
                    onClick = { onSelectConnection(connection.id) },
                )
                Spacer(Modifier.height(4.dp))
            }
        }

        Spacer(Modifier.height(12.dp))
        HorizontalDivider(color = Border)
        Spacer(Modifier.height(12.dp))

        Text(
            text = "РАБОЧАЯ ОБЛАСТЬ",
            color = TextSecondary,
            fontSize = 10.sp,
            fontWeight = FontWeight.Bold,
            letterSpacing = 1.1.sp,
            modifier = Modifier.padding(horizontal = 10.dp),
        )
        Spacer(Modifier.height(8.dp))

        AppDestination.entries.filter(AppDestination::visibleInSidebar).forEach { item ->
            NavigationItem(
                destination = item,
                selected = destination == item,
                onClick = { onNavigate(item) },
            )
            Spacer(Modifier.height(3.dp))
        }

        Spacer(Modifier.weight(1f))
        HorizontalDivider(color = Border)
        Spacer(Modifier.height(12.dp))
        Text("Поддержка Camunda", color = TextSecondary, fontSize = 11.sp)
        Text("Предварительная версия · 1.0.0", color = TextSecondary.copy(alpha = 0.65f), fontSize = 10.sp)
    }
}

@Composable
private fun Brand() {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(
            modifier = Modifier.size(34.dp).background(Primary, RoundedCornerShape(10.dp)),
            contentAlignment = Alignment.Center,
        ) {
            Text("C", color = Color.White, fontWeight = FontWeight.Black, fontSize = 17.sp)
        }
        Spacer(Modifier.width(10.dp))
        Column {
            Text("CAMUNDA", color = TextPrimary, fontWeight = FontWeight.Bold, fontSize = 13.sp, letterSpacing = 0.8.sp)
            Text("ПОДДЕРЖКА", color = TextSecondary, fontSize = 9.sp, letterSpacing = 0.9.sp)
        }
    }
}

@Composable
private fun ConnectionItem(
    connection: CamundaConnection,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val background = if (selected) PrimaryMuted else Color.Transparent
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(background)
            .clickable(onClick = onClick)
            .padding(horizontal = 10.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier
                .size(7.dp)
                .background(environmentColor(connection.environment), CircleShape),
        )
        Spacer(Modifier.width(9.dp))
        Column(Modifier.weight(1f)) {
            Text(
                text = connection.name,
                color = if (selected) TextPrimary else TextSecondary,
                fontSize = 12.sp,
                fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = connection.environment.label,
                color = TextSecondary.copy(alpha = 0.68f),
                fontSize = 9.sp,
            )
        }
    }
}

@Composable
private fun NavigationItem(
    destination: AppDestination,
    selected: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(9.dp))
            .background(if (selected) PrimaryMuted else Color.Transparent)
            .clickable(onClick = onClick)
            .padding(horizontal = 10.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = destination.marker,
            color = if (selected) Primary else TextSecondary.copy(alpha = 0.7f),
            fontFamily = FontFamily.Monospace,
            fontSize = 10.sp,
        )
        Spacer(Modifier.width(10.dp))
        Text(
            text = destination.title,
            color = if (selected) TextPrimary else TextSecondary,
            fontSize = 12.sp,
            fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
        )
    }
}
