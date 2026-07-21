package com.malinatrash.camundasupport

import androidx.compose.ui.unit.dp
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.awt.ComposeWindow
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import com.malinatrash.camundasupport.data.DesktopExternalNavigator
import com.malinatrash.camundasupport.data.DesktopAppUpdateService
import com.malinatrash.camundasupport.data.DesktopConnectionTester
import com.malinatrash.camundasupport.data.DesktopDeepLinkSource
import com.malinatrash.camundasupport.data.DesktopCamundaApi
import com.malinatrash.camundasupport.data.DesktopProtocolRegistrar
import com.malinatrash.camundasupport.data.DesktopTextClipboard
import com.malinatrash.camundasupport.data.APP_BUILD
import com.malinatrash.camundasupport.data.APP_VERSION
import com.malinatrash.camundasupport.data.AppThemeMode
import com.malinatrash.camundasupport.data.PreferencesConnectionRepository
import com.malinatrash.camundasupport.data.PreferencesVariableKeyRepository
import com.malinatrash.camundasupport.data.PreferencesThemeRepository
import javax.swing.JRootPane
import javax.swing.SwingUtilities
import java.awt.Frame

fun main(args: Array<String>) {
    val themeRepository = PreferencesThemeRepository()
    configureNativeApplicationTheme(themeRepository.load())
    DesktopProtocolRegistrar().register()
    val deepLinkSource = DesktopDeepLinkSource(args.toList()).apply { registerSystemHandler() }

    application {
        Window(
            onCloseRequest = ::exitApplication,
            title = "Поддержка Camunda · версия $APP_VERSION · сборка $APP_BUILD",
            state = rememberWindowState(width = 1440.dp, height = 900.dp),
        ) {
            SideEffect {
                deepLinkSource.setActivationHandler { activateWindow(window) }
            }
            CamundaSupportApp(
                connectionRepository = PreferencesConnectionRepository(),
                connectionTester = DesktopConnectionTester(),
                camundaApi = DesktopCamundaApi(),
                variableKeyRepository = PreferencesVariableKeyRepository(),
                themeRepository = themeRepository,
                deepLinkSource = deepLinkSource,
                textClipboard = DesktopTextClipboard(),
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

private fun activateWindow(window: ComposeWindow) {
    SwingUtilities.invokeLater {
        window.extendedState = Frame.NORMAL
        window.isVisible = true
        window.toFront()
        window.requestFocus()
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
