package com.malinatrash.camundasupport.data

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class AppUpdateServiceTest {
    @Test
    fun comparesSemanticVersionsNumerically() {
        assertTrue(isNewerVersion("v1.2.0", "1.1.9"))
        assertTrue(isNewerVersion("2.0", "1.99.99"))
        assertFalse(isNewerVersion("v1.2.0", "1.2.0"))
        assertFalse(isNewerVersion("1.1.9", "1.2.0"))
    }
}
