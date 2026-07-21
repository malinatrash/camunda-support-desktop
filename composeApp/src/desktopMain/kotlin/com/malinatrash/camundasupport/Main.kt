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
import com.malinatrash.camundasupport.data.PreferencesConnectionRepository
import com.malinatrash.camundasupport.data.PreferencesVariableKeyRepository

fun main() = application {
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
            externalNavigator = DesktopExternalNavigator(),
            updateService = DesktopAppUpdateService(),
        )
    }
}
