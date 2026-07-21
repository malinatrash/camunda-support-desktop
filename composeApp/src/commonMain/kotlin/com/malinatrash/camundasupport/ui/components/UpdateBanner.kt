package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.data.AppUpdate
import com.malinatrash.camundasupport.data.UpdateDownloadProgress
import com.malinatrash.camundasupport.data.UpdateDownloadStage
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.PrimaryMuted
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun UpdateBanner(
    update: AppUpdate,
    downloading: Boolean,
    progress: UpdateDownloadProgress?,
    error: String?,
    openedInstaller: String?,
    onOpenRelease: () -> Unit,
    onInstall: () -> Unit,
    onDismiss: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().background(PrimaryMuted).padding(horizontal = 20.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(
                "Доступна новая версия ${update.version}",
                color = TextPrimary,
                fontWeight = FontWeight.SemiBold,
                fontSize = 12.sp,
            )
            val status = when {
                openedInstaller != null -> "Установщик $openedInstaller открыт. Завершите установку в системном окне."
                error != null -> error
                update.asset == null -> "Для этой платформы нет автоматического установщика. Откройте страницу релиза."
                downloading && progress != null -> progress.stage.label()
                else -> "Скачаем установщик с GitHub, проверим SHA-256 и откроем его."
            }
            Text(status, color = if (error == null) TextSecondary else Danger, fontSize = 10.sp)

            update.asset?.takeIf { downloading || progress != null }?.let { asset ->
                Spacer(Modifier.height(5.dp))
                SelectionContainer {
                    Column {
                        Text(
                            "Версия: ${update.version} · Тег: ${update.tag} · Файл: ${asset.name}",
                            color = TextSecondary,
                            fontSize = 9.sp,
                        )
                        Text(
                            "Размер: ${asset.sizeBytes.formatBytes()} (${asset.sizeBytes} байт)",
                            color = TextSecondary,
                            fontSize = 9.sp,
                        )
                        Text(
                            "SHA-256: ${asset.digest ?: update.checksumUrl ?: "не указан"}",
                            color = TextSecondary,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 9.sp,
                        )
                        Text(
                            "Релиз: ${update.releaseUrl}",
                            color = TextSecondary,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 9.sp,
                        )
                        update.checksumUrl?.let { checksumUrl ->
                            Text(
                                "Контрольные суммы: $checksumUrl",
                                color = TextSecondary,
                                fontFamily = FontFamily.Monospace,
                                fontSize = 9.sp,
                            )
                        }
                        Text(
                            "Источник: ${asset.downloadUrl}",
                            color = TextSecondary,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 9.sp,
                        )
                        progress?.let {
                            Text(
                                "Сохранение: ${it.destinationPath}",
                                color = TextSecondary,
                                fontFamily = FontFamily.Monospace,
                                fontSize = 9.sp,
                            )
                        }
                    }
                }
            }

            if (downloading && progress != null) {
                Spacer(Modifier.height(7.dp))
                val fraction = progress.fraction()
                if (fraction == null) {
                    LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                } else {
                    LinearProgressIndicator(progress = { fraction }, modifier = Modifier.fillMaxWidth())
                }
                Spacer(Modifier.height(4.dp))
                Row(Modifier.fillMaxWidth()) {
                    Text(
                        "${progress.downloadedBytes.formatBytes()} из ${progress.totalBytes.formatBytes()}" +
                            if (progress.bytesPerSecond > 0) " · ${progress.bytesPerSecond.formatBytes()}/с" else "",
                        color = TextSecondary,
                        fontSize = 9.sp,
                    )
                    Spacer(Modifier.weight(1f))
                    Text(
                        progress.progressLabel(),
                        color = TextPrimary,
                        fontSize = 9.sp,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }
        }
        TextButton(onClick = onOpenRelease) { Text("Что нового") }
        Spacer(Modifier.width(6.dp))
        Button(onClick = onInstall, enabled = !downloading && openedInstaller == null) {
            Text(
                when {
                    downloading -> "Скачиваем…"
                    openedInstaller != null -> "Установщик открыт"
                    update.asset == null -> "Открыть релиз"
                    else -> "Скачать и установить"
                },
            )
        }
        Spacer(Modifier.width(4.dp))
        TextButton(onClick = onDismiss, enabled = !downloading) { Text("Позже") }
    }
}

private fun UpdateDownloadProgress.fraction(): Float? = totalBytes
    .takeIf { it > 0 }
    ?.let { (downloadedBytes.toDouble() / it.toDouble()).toFloat().coerceIn(0f, 1f) }

private fun UpdateDownloadProgress.progressLabel(): String {
    val percentage = fraction()?.let { "${(it * 100).toInt()}%" } ?: "—"
    val remaining = (totalBytes - downloadedBytes).coerceAtLeast(0)
    val remainingTime = bytesPerSecond
        .takeIf { it > 0 }
        ?.let { speed -> (remaining / speed).coerceAtLeast(0).formatDuration() }
    return if (stage == UpdateDownloadStage.Downloading && totalBytes > 0) {
        "$percentage · осталось ${remaining.formatBytes()}${remainingTime?.let { " · ≈ $it" }.orEmpty()}"
    } else {
        stage.label()
    }
}

private fun UpdateDownloadStage.label(): String = when (this) {
    UpdateDownloadStage.Preparing -> "Подготавливаем загрузку…"
    UpdateDownloadStage.Downloading -> "Загружаем установщик…"
    UpdateDownloadStage.Verifying -> "Проверяем SHA-256…"
    UpdateDownloadStage.OpeningInstaller -> "Открываем установщик…"
}

private fun Long.formatDuration(): String = if (this < 60) {
    "$this с"
} else {
    "${this / 60} мин ${this % 60} с"
}

private fun Long.formatBytes(): String {
    if (this <= 0) return "0 Б"
    val units = listOf("Б", "КБ", "МБ", "ГБ")
    var value = toDouble()
    var unitIndex = 0
    while (value >= 1024 && unitIndex < units.lastIndex) {
        value /= 1024
        unitIndex += 1
    }
    if (unitIndex == 0) return "$this ${units[unitIndex]}"
    val tenths = (value * 10).toInt()
    val decimal = tenths % 10
    return if (decimal == 0) {
        "${tenths / 10} ${units[unitIndex]}"
    } else {
        "${tenths / 10},$decimal ${units[unitIndex]}"
    }
}
