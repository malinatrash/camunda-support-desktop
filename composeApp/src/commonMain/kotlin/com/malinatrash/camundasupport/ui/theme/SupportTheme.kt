package com.malinatrash.camundasupport.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import com.malinatrash.camundasupport.data.AppThemeMode

@Immutable
internal data class SupportPalette(
    val appBackground: Color,
    val sidebarBackground: Color,
    val surface: Color,
    val surfaceElevated: Color,
    val border: Color,
    val primary: Color,
    val primaryMuted: Color,
    val selectionBorder: Color,
    val textPrimary: Color,
    val textSecondary: Color,
    val healthy: Color,
    val warning: Color,
    val danger: Color,
    val development: Color,
)

private val LightPalette = SupportPalette(
    appBackground = Color(0xFFF4F7F4),
    sidebarBackground = Color(0xFFFFFFFF),
    surface = Color(0xFFFFFFFF),
    surfaceElevated = Color(0xFFF9FBF9),
    border = Color(0xFFDCE5DC),
    primary = Color(0xFF00CD00),
    primaryMuted = Color(0xFFE4FBE4),
    selectionBorder = Color(0xFF00CD00),
    textPrimary = Color(0xFF172017),
    textSecondary = Color(0xFF647064),
    healthy = Color(0xFF16896F),
    warning = Color(0xFFB56A00),
    danger = Color(0xFFD92D20),
    development = Color(0xFF2474C8),
)

private val DarkPalette = SupportPalette(
    appBackground = Color(0xFF0F1115),
    sidebarBackground = Color(0xFF15181D),
    surface = Color(0xFF1B1F25),
    surfaceElevated = Color(0xFF22272E),
    border = Color(0xFF303742),
    primary = Color(0xFF00CD00),
    primaryMuted = Color(0xFF282E38),
    selectionBorder = Color(0xFF566273),
    textPrimary = Color(0xFFEDF1F7),
    textSecondary = Color(0xFF98A2B3),
    healthy = Color(0xFF25C07A),
    warning = Color(0xFFF1A340),
    danger = Color(0xFFFF6B61),
    development = Color(0xFF6BA8E8),
)

private val LocalSupportPalette = staticCompositionLocalOf { LightPalette }

val AppBackground: Color
    @Composable get() = LocalSupportPalette.current.appBackground
val SidebarBackground: Color
    @Composable get() = LocalSupportPalette.current.sidebarBackground
val Surface: Color
    @Composable get() = LocalSupportPalette.current.surface
val SurfaceElevated: Color
    @Composable get() = LocalSupportPalette.current.surfaceElevated
val Border: Color
    @Composable get() = LocalSupportPalette.current.border
val Primary: Color
    @Composable get() = LocalSupportPalette.current.primary
val PrimaryMuted: Color
    @Composable get() = LocalSupportPalette.current.primaryMuted
val SelectionBorder: Color
    @Composable get() = LocalSupportPalette.current.selectionBorder
val TextPrimary: Color
    @Composable get() = LocalSupportPalette.current.textPrimary
val TextSecondary: Color
    @Composable get() = LocalSupportPalette.current.textSecondary
val Healthy: Color
    @Composable get() = LocalSupportPalette.current.healthy
val Warning: Color
    @Composable get() = LocalSupportPalette.current.warning
val Danger: Color
    @Composable get() = LocalSupportPalette.current.danger
val Development: Color
    @Composable get() = LocalSupportPalette.current.development

internal fun paletteFor(themeMode: AppThemeMode): SupportPalette = when (themeMode) {
    AppThemeMode.Light -> LightPalette
    AppThemeMode.Dark -> DarkPalette
}

@Composable
fun SupportTheme(
    themeMode: AppThemeMode = AppThemeMode.Light,
    content: @Composable () -> Unit,
) {
    val palette = paletteFor(themeMode)
    val colorScheme = if (themeMode == AppThemeMode.Dark) {
        darkColorScheme(
            primary = palette.primary,
            onPrimary = Color(0xFF041004),
            primaryContainer = palette.primaryMuted,
            onPrimaryContainer = palette.textPrimary,
            secondary = palette.healthy,
            onSecondary = Color(0xFF041004),
            background = palette.appBackground,
            onBackground = palette.textPrimary,
            surface = palette.surface,
            onSurface = palette.textPrimary,
            surfaceVariant = palette.surfaceElevated,
            onSurfaceVariant = palette.textSecondary,
            outline = palette.border,
            error = palette.danger,
            onError = Color(0xFF180303),
        )
    } else {
        lightColorScheme(
            primary = palette.primary,
            onPrimary = Color(0xFF041004),
            primaryContainer = palette.primaryMuted,
            onPrimaryContainer = Color(0xFF075207),
            secondary = palette.healthy,
            onSecondary = Color.White,
            background = palette.appBackground,
            onBackground = palette.textPrimary,
            surface = palette.surface,
            onSurface = palette.textPrimary,
            surfaceVariant = palette.surfaceElevated,
            onSurfaceVariant = palette.textSecondary,
            outline = palette.border,
            error = palette.danger,
            onError = Color.White,
        )
    }

    CompositionLocalProvider(
        LocalSupportPalette provides palette,
        LocalContentColor provides palette.textPrimary,
    ) {
        MaterialTheme(
            colorScheme = colorScheme,
            content = content,
        )
    }
}
