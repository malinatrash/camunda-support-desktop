package com.malinatrash.camundasupport.data

import java.util.UUID
import java.util.prefs.Preferences
import kotlin.test.Test
import kotlin.test.assertEquals

class PreferencesThemeRepositoryTest {
    @Test
    fun themeSurvivesRepositoryRecreation() {
        val preferences = Preferences.userRoot().node(
            "com/malinatrash/camunda-support/tests/theme-${UUID.randomUUID()}",
        )
        try {
            assertEquals(AppThemeMode.Light, PreferencesThemeRepository(preferences).load())

            PreferencesThemeRepository(preferences).save(AppThemeMode.Dark)

            assertEquals(AppThemeMode.Dark, PreferencesThemeRepository(preferences).load())
        } finally {
            preferences.removeNode()
        }
    }
}
