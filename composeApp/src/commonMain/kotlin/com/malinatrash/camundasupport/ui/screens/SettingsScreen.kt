package com.malinatrash.camundasupport.ui.screens

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
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.data.APP_BUILD
import com.malinatrash.camundasupport.data.APP_VERSION
import com.malinatrash.camundasupport.data.AppThemeMode
import com.malinatrash.camundasupport.data.ReleaseDownloadStats
import com.malinatrash.camundasupport.data.ReleaseDownloadStatsResult
import com.malinatrash.camundasupport.ui.components.SectionTitle
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.SelectionBorder
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.SurfaceElevated
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary
import com.malinatrash.camundasupport.ui.theme.paletteFor

@Composable
fun SettingsScreen(
    selectedTheme: AppThemeMode,
    downloadStatsResult: ReleaseDownloadStatsResult?,
    downloadStatsLoading: Boolean,
    onThemeSelected: (AppThemeMode) -> Unit,
    onRefreshDownloadStats: () -> Unit,
) {
    Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(bottom = 16.dp)) {
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

        Spacer(Modifier.height(16.dp))
        AboutApplicationCard(
            downloadStatsResult = downloadStatsResult,
            downloadStatsLoading = downloadStatsLoading,
            onRefreshDownloadStats = onRefreshDownloadStats,
        )
    }
}

@Composable
private fun AboutApplicationCard(
    downloadStatsResult: ReleaseDownloadStatsResult?,
    downloadStatsLoading: Boolean,
    onRefreshDownloadStats: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(14.dp),
    ) {
        Column(Modifier.fillMaxWidth().padding(18.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column {
                    Text("О приложении", color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                    Spacer(Modifier.height(3.dp))
                    Text(
                        "Версия $APP_VERSION · сборка $APP_BUILD",
                        color = TextSecondary,
                        fontSize = 11.sp,
                    )
                }
                OutlinedButton(
                    onClick = onRefreshDownloadStats,
                    enabled = !downloadStatsLoading,
                ) {
                    Text(if (downloadStatsLoading) "Обновляем…" else "Обновить статистику")
                }
            }

            Spacer(Modifier.height(14.dp))
            when (downloadStatsResult) {
                null -> Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                    Text("Считаем скачивания всех релизов GitHub…", color = TextSecondary, fontSize = 11.sp)
                }

                is ReleaseDownloadStatsResult.Failure -> Column {
                    Text(downloadStatsResult.message, color = Danger, fontSize = 12.sp)
                    Spacer(Modifier.height(4.dp))
                    Text(
                        "Проверьте доступ к api.github.com и повторите загрузку.",
                        color = TextSecondary,
                        fontSize = 11.sp,
                    )
                }

                is ReleaseDownloadStatsResult.Success -> DownloadStatsContent(downloadStatsResult.stats)
            }
        }
    }
}

@Composable
private fun DownloadStatsContent(stats: ReleaseDownloadStats) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(SurfaceElevated, RoundedCornerShape(10.dp))
                .padding(horizontal = 16.dp, vertical = 14.dp),
        ) {
            Text("ВСЕГО СКАЧИВАНИЙ", color = TextSecondary, fontSize = 10.sp, fontWeight = FontWeight.Bold)
            Text(
                stats.totalDownloads.grouped(),
                color = Primary,
                fontSize = 30.sp,
                fontWeight = FontWeight.Bold,
            )
            Text("Установщики всех опубликованных версий", color = TextSecondary, fontSize = 11.sp)
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            DownloadMetric("macOS", stats.macOsDownloads, Modifier.weight(1f))
            DownloadMetric("Windows", stats.windowsDownloads, Modifier.weight(1f))
            DownloadMetric("Linux", stats.linuxDownloads, Modifier.weight(1f))
        }
        Text(
            "Учтено релизов: ${stats.releaseCount} · установочных файлов: ${stats.installerAssetCount}. " +
                "Повторные скачивания также входят в счётчик.",
            color = TextSecondary,
            fontSize = 10.sp,
        )
    }
}

@Composable
private fun DownloadMetric(label: String, value: Long, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier
            .background(SurfaceElevated, RoundedCornerShape(9.dp))
            .padding(12.dp),
    ) {
        Text(label, color = TextSecondary, fontSize = 10.sp)
        Spacer(Modifier.height(2.dp))
        Text(value.grouped(), color = TextPrimary, fontSize = 18.sp, fontWeight = FontWeight.Bold)
    }
}

private fun Long.grouped(): String = toString()
    .reversed()
    .chunked(3)
    .joinToString(" ")
    .reversed()

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
        border = BorderStroke(if (selected) 2.dp else 1.dp, if (selected) SelectionBorder else Border),
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
