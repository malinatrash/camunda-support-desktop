package com.malinatrash.camundasupport

import com.malinatrash.camundasupport.data.AppThemeMode
import javax.swing.JRootPane
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class NativeWindowThemeTest {
    @Test
    fun `macOS window follows selected theme`() {
        val rootPane = JRootPane()

        applyNativeWindowTheme(rootPane, AppThemeMode.Dark, osName = "Mac OS X")
        assertEquals("NSAppearanceNameDarkAqua", rootPane.getClientProperty("apple.awt.windowAppearance"))

        applyNativeWindowTheme(rootPane, AppThemeMode.Light, osName = "Mac OS X")
        assertEquals("NSAppearanceNameAqua", rootPane.getClientProperty("apple.awt.windowAppearance"))
    }

    @Test
    fun `other operating systems do not receive macOS property`() {
        val rootPane = JRootPane()

        applyNativeWindowTheme(rootPane, AppThemeMode.Dark, osName = "Windows 11")

        assertNull(rootPane.getClientProperty("apple.awt.windowAppearance"))
    }
}
