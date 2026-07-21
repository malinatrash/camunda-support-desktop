package com.malinatrash.camundasupport.data

enum class AppThemeMode(val label: String) {
    Light("Светлая"),
    Dark("Тёмная"),
}

interface ThemeRepository {
    fun load(): AppThemeMode

    fun save(themeMode: AppThemeMode)
}

class InMemoryThemeRepository(
    initialTheme: AppThemeMode = AppThemeMode.Light,
) : ThemeRepository {
    private var themeMode = initialTheme

    override fun load(): AppThemeMode = themeMode

    override fun save(themeMode: AppThemeMode) {
        this.themeMode = themeMode
    }
}
