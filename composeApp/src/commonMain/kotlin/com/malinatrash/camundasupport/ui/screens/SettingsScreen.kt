package com.malinatrash.camundasupport.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.data.AppThemeMode
import com.malinatrash.camundasupport.ui.components.SectionTitle
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary
import com.malinatrash.camundasupport.ui.theme.paletteFor

@Composable
fun SettingsScreen(
    selectedTheme: AppThemeMode,
    onThemeSelected: (AppThemeMode) -> Unit,
) {
    Column {
        SectionTitle(
            title = "Настройки",
            description = "Оформление и локальные параметры приложения.",
        )
        Spacer(Modifier.height(16.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Surface),
            border = BorderStroke(1.dp, Border),
            shape = RoundedCornerShape(14.dp),
        ) {
            Column(Modifier.fillMaxWidth().padding(18.dp)) {
                Text("Цветовая тема", color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.height(3.dp))
                Text(
                    "Выбор сохраняется на этом компьютере и применяется сразу. По умолчанию используется светлая тема.",
                    color = TextSecondary,
                    fontSize = 11.sp,
                )
                Spacer(Modifier.height(14.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    AppThemeMode.entries.forEach { themeMode ->
                        ThemeOption(
                            themeMode = themeMode,
                            selected = themeMode == selectedTheme,
                            onClick = { onThemeSelected(themeMode) },
                            modifier = Modifier.weight(1f),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ThemeOption(
    themeMode: AppThemeMode,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val preview = paletteFor(themeMode)
    Card(
        modifier = modifier.clickable(onClick = onClick),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(if (selected) 2.dp else 1.dp, if (selected) Primary else Border),
        shape = RoundedCornerShape(12.dp),
    ) {
        Column(Modifier.fillMaxWidth().padding(12.dp)) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(72.dp)
                    .background(preview.appBackground, RoundedCornerShape(8.dp))
                    .padding(9.dp),
            ) {
                Row(Modifier.fillMaxWidth()) {
                    Box(
                        Modifier
                            .size(width = 32.dp, height = 54.dp)
                            .background(preview.sidebarBackground, RoundedCornerShape(5.dp)),
                    )
                    Spacer(Modifier.size(7.dp))
                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Box(
                            Modifier
                                .fillMaxWidth()
                                .height(12.dp)
                                .background(preview.surface, RoundedCornerShape(4.dp)),
                        )
                        Box(
                            Modifier
                                .fillMaxWidth(0.62f)
                                .height(8.dp)
                                .background(Color(0xFF00CD00), RoundedCornerShape(4.dp)),
                        )
                        Box(
                            Modifier
                                .fillMaxWidth()
                                .height(20.dp)
                                .background(preview.surfaceElevated, RoundedCornerShape(4.dp)),
                        )
                    }
                }
            }
            Spacer(Modifier.height(8.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                RadioButton(selected = selected, onClick = onClick)
                Column {
                    Text(themeMode.label, color = TextPrimary, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                    Text(
                        if (themeMode == AppThemeMode.Light) "Светлый фон" else "Тёмный фон",
                        color = TextSecondary,
                        fontSize = 10.sp,
                    )
                }
            }
        }
    }
}
