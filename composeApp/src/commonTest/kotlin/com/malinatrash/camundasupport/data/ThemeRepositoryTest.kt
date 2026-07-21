package com.malinatrash.camundasupport.data

import androidx.compose.ui.graphics.Color
import com.malinatrash.camundasupport.ui.theme.paletteFor
import kotlin.test.Test
import kotlin.test.assertEquals

class ThemeRepositoryTest {
    @Test
    fun bothThemesUseRequiredGreenAccent() {
        val requiredAccent = Color(0xFF00CD00)

        assertEquals(requiredAccent, paletteFor(AppThemeMode.Light).primary)
        assertEquals(requiredAccent, paletteFor(AppThemeMode.Dark).primary)
    }

    @Test
    fun `dark selection surfaces and borders are neutral`() {
        val darkPalette = paletteFor(AppThemeMode.Dark)

        assertEquals(Color(0xFF282E38), darkPalette.primaryMuted)
        assertEquals(Color(0xFF566273), darkPalette.selectionBorder)
    }

    @Test
    fun lightThemeIsDefaultAndSelectionIsRemembered() {
        val repository = InMemoryThemeRepository()

        assertEquals(AppThemeMode.Light, repository.load())

        repository.save(AppThemeMode.Dark)

        assertEquals(AppThemeMode.Dark, repository.load())
    }
}
