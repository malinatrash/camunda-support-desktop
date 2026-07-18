package com.malinatrash.camundasupport.model

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ConnectionValidationTest {
    @Test
    fun validHttpsConnectionPassesValidation() {
        val validation = validateConnectionDraft(
            ConnectionDraft(
                name = "B2C Prod",
                restUrl = "https://camunda.example.kz/engine-rest",
                cockpitUrl = "https://camunda.example.kz/camunda/app/cockpit",
                environment = Environment.Production,
            ),
        )

        assertTrue(validation.isValid)
    }

    @Test
    fun invalidConnectionReturnsFieldErrors() {
        val validation = validateConnectionDraft(
            ConnectionDraft(
                name = " ",
                restUrl = "camunda.internal/engine-rest",
            ),
        )

        assertFalse(validation.isValid)
        assertEquals("Укажите название", validation.nameError)
        assertEquals("URL должен начинаться с https:// или http://", validation.restUrlError)
    }

    @Test
    fun restUrlMustEndWithEngineRest() {
        val invalidUrls = listOf(
            "https://camunda.example.kz",
            "https://camunda.example.kz/engine",
            "https://camunda.example.kz/engine-rest/version",
            "https://camunda.example.kz/ENGINE-REST",
        )

        invalidUrls.forEach { restUrl ->
            val validation = validateConnectionDraft(
                ConnectionDraft(name = "Test", restUrl = restUrl),
            )

            assertFalse(validation.isValid, restUrl)
            assertEquals("Адрес Camunda REST должен заканчиваться на /engine-rest", validation.restUrlError)
        }
    }

    @Test
    fun engineRestUrlAllowsOneTrailingSlash() {
        val validation = validateConnectionDraft(
            ConnectionDraft(
                name = "Test",
                restUrl = "https://camunda.example.kz/engine-rest/",
            ),
        )

        assertTrue(validation.isValid)
    }

    @Test
    fun restUrlRejectsQueryAndFragment() {
        val validation = validateConnectionDraft(
            ConnectionDraft(
                name = "Test",
                restUrl = "https://camunda.example.kz/engine-rest?tenant=b2c",
            ),
        )

        assertFalse(validation.isValid)
        assertEquals("URL не должен содержать query-параметры или fragment", validation.restUrlError)
    }
}
