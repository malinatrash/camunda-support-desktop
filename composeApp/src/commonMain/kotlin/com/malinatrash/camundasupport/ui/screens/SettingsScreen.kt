package com.malinatrash.camundasupport.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.malinatrash.camundasupport.ui.components.EmptyPanel
import com.malinatrash.camundasupport.ui.components.SectionTitle

@Composable
fun SettingsScreen() {
    Column {
        SectionTitle(
            title = "Настройки",
            description = "Оформление, маскирование, локальный аудит и диагностика приложения.",
        )
        Spacer(Modifier.height(12.dp))
        EmptyPanel(
            title = "Включены безопасные настройки по умолчанию",
            description = "Подтверждение операций в продакшене, маскирование переменных и безопасное хранение авторизации появятся на следующих этапах.",
        )
    }
}
