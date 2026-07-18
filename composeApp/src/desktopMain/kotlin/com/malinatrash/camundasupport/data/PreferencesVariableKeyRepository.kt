package com.malinatrash.camundasupport.data

import java.util.prefs.Preferences

class PreferencesVariableKeyRepository(
    private val root: Preferences = Preferences.userRoot().node("com/malinatrash/camunda-support/variable-keys"),
    private val legacyRoot: Preferences? = Preferences.userRoot().node("kz/bereke/camunda-support/variable-keys"),
) : VariableKeyRepository {
    init {
        migrateLegacyKeys()
    }

    override fun load(connectionId: String): List<String> = root
        .node(connectionId)
        .get(KEYS, "")
        .lineSequence()
        .map(String::trim)
        .filter(String::isNotEmpty)
        .distinct()
        .sortedBy(String::lowercase)
        .toList()

    override fun save(connectionId: String, key: String) {
        val normalized = key.trim()
        if (normalized.isEmpty()) return
        val node = root.node(connectionId)
        val updated = (load(connectionId) + normalized)
            .distinct()
            .sortedBy(String::lowercase)
        node.put(KEYS, updated.joinToString("\n"))
        node.flush()
    }

    private fun migrateLegacyKeys() {
        val legacy = legacyRoot ?: return
        if (root.childrenNames().isNotEmpty()) return
        legacy.childrenNames().forEach { connectionId ->
            val source = legacy.node(connectionId)
            val target = root.node(connectionId)
            source.keys().forEach { key -> target.put(key, source.get(key, "")) }
        }
        root.flush()
    }

    private companion object {
        const val KEYS = "keys"
    }
}
