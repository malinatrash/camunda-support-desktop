package com.malinatrash.camundasupport

import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import com.malinatrash.camundasupport.data.DesktopExternalNavigator
import com.malinatrash.camundasupport.data.DesktopAppUpdateService
import com.malinatrash.camundasupport.data.DesktopConnectionTester
import com.malinatrash.camundasupport.data.DesktopCamundaApi
import com.malinatrash.camundasupport.data.APP_BUILD
import com.malinatrash.camundasupport.data.APP_VERSION
import com.malinatrash.camundasupport.data.AppThemeMode
import com.malinatrash.camundasupport.data.PreferencesConnectionRepository
import com.malinatrash.camundasupport.data.PreferencesVariableKeyRepository
import com.malinatrash.camundasupport.data.PreferencesThemeRepository
import javax.swing.JRootPane
import javax.swing.SwingUtilities

fun main() {
    val themeRepository = PreferencesThemeRepository()
    configureNativeApplicationTheme(themeRepository.load())

    application {
        Window(
            onCloseRequest = ::exitApplication,
            title = "Поддержка Camunda · версия $APP_VERSION · сборка $APP_BUILD",
            state = rememberWindowState(width = 1440.dp, height = 900.dp),
        ) {
            CamundaSupportApp(
                connectionRepository = PreferencesConnectionRepository(),
                connectionTester = DesktopConnectionTester(),
                camundaApi = DesktopCamundaApi(),
                variableKeyRepository = PreferencesVariableKeyRepository(),
                themeRepository = themeRepository,
                onThemeChanged = { themeMode ->
                    val rootPane = window.rootPane
                    if (SwingUtilities.isEventDispatchThread()) {
                        applyNativeWindowTheme(rootPane, themeMode)
                    } else {
                        SwingUtilities.invokeLater { applyNativeWindowTheme(rootPane, themeMode) }
                    }
                },
                externalNavigator = DesktopExternalNavigator(),
                updateService = DesktopAppUpdateService(),
            )
        }
    }
}

internal fun configureNativeApplicationTheme(
    themeMode: AppThemeMode,
    osName: String = System.getProperty("os.name").orEmpty(),
) {
    if (!osName.isMacOs()) return
    System.setProperty(MACOS_APPLICATION_APPEARANCE_PROPERTY, themeMode.nativeMacOsAppearance())
}

internal fun applyNativeWindowTheme(
    rootPane: JRootPane,
    themeMode: AppThemeMode,
    osName: String = System.getProperty("os.name").orEmpty(),
) {
    if (!osName.isMacOs()) return
    rootPane.putClientProperty(MACOS_WINDOW_APPEARANCE_PROPERTY, themeMode.nativeMacOsAppearance())
    rootPane.revalidate()
    rootPane.repaint()
}

private fun String.isMacOs(): Boolean = contains("mac", ignoreCase = true)

private fun AppThemeMode.nativeMacOsAppearance(): String = when (this) {
    AppThemeMode.Light -> "NSAppearanceNameAqua"
    AppThemeMode.Dark -> "NSAppearanceNameDarkAqua"
}

private const val MACOS_APPLICATION_APPEARANCE_PROPERTY = "apple.awt.application.appearance"
private const val MACOS_WINDOW_APPEARANCE_PROPERTY = "apple.awt.windowAppearance"
