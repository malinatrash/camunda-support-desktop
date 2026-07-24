package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.ProcessVariableCatalog
import com.malinatrash.camundasupport.model.ProcessVariableDescriptor
import java.util.UUID
import java.util.prefs.Preferences
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PreferencesProcessVariableCatalogRepositoryTest {
    @Test
    fun persistsCatalogBetweenRepositoryInstancesAndRemovesIt() {
        val root = Preferences.userRoot().node(
            "com/malinatrash/camunda-support/tests/process-catalog-${UUID.randomUUID()}",
        )
        try {
            val stored = StoredProcessVariableCatalog(
                catalog = ProcessVariableCatalog(
                    processDefinitionKey = "loan",
                    variables = listOf(
                        ProcessVariableDescriptor("applicationId", listOf("String"), 12),
                        ProcessVariableDescriptor("amount", listOf("Double", "Long"), 7),
                    ),
                    inspectedInstanceCount = 42,
                    instancesTruncated = true,
                ),
                loadedAtEpochMillis = 1_721_779_200_000,
            )

            PreferencesProcessVariableCatalogRepository(root).save("https://example.test/engine-rest", stored)
            val reloaded = PreferencesProcessVariableCatalogRepository(root)
                .load("https://example.test/engine-rest", "loan")

            assertEquals(stored, reloaded)

            PreferencesProcessVariableCatalogRepository(root)
                .remove("https://example.test/engine-rest", "loan")
            assertNull(
                PreferencesProcessVariableCatalogRepository(root)
                    .load("https://example.test/engine-rest", "loan"),
            )
        } finally {
            root.removeNode()
        }
    }
}
