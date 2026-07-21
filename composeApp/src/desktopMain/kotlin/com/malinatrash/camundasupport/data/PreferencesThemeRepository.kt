package com.malinatrash.camundasupport.data

import java.util.prefs.Preferences

class PreferencesThemeRepository(
    private val preferences: Preferences = Preferences.userRoot().node("com/malinatrash/camunda-support/settings"),
) : ThemeRepository {
    override fun load(): AppThemeMode {
        val storedValue = preferences.get(THEME_KEY, AppThemeMode.Light.name)
        return AppThemeMode.entries.firstOrNull { it.name == storedValue } ?: AppThemeMode.Light
    }

    override fun save(themeMode: AppThemeMode) {
        preferences.put(THEME_KEY, themeMode.name)
        preferences.flush()
    }

    private companion object {
        const val THEME_KEY = "theme"
    }
}
