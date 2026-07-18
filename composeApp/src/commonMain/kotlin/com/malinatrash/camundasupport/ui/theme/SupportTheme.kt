package com.malinatrash.camundasupport.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

val AppBackground = Color(0xFFF5F7FB)
val SidebarBackground = Color(0xFFFFFFFF)
val Surface = Color(0xFFFFFFFF)
val SurfaceElevated = Color(0xFFFFFFFF)
val Border = Color(0xFFDDE3EC)
val Primary = Color(0xFF4556D7)
val PrimaryMuted = Color(0xFFE9ECFF)
val TextPrimary = Color(0xFF172033)
val TextSecondary = Color(0xFF667085)
val Healthy = Color(0xFF16896F)
val Warning = Color(0xFFB56A00)
val Danger = Color(0xFFD92D20)
val Development = Color(0xFF2474C8)

private val SupportColorScheme = lightColorScheme(
    primary = Primary,
    onPrimary = Color.White,
    primaryContainer = PrimaryMuted,
    onPrimaryContainer = Color(0xFF26318B),
    secondary = Healthy,
    onSecondary = Color.White,
    background = AppBackground,
    onBackground = TextPrimary,
    surface = Surface,
    onSurface = TextPrimary,
    surfaceVariant = Color(0xFFEEF2F7),
    onSurfaceVariant = TextSecondary,
    outline = Border,
    error = Danger,
    onError = Color.White,
)

@Composable
fun SupportTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = SupportColorScheme,
        content = content,
    )
}
