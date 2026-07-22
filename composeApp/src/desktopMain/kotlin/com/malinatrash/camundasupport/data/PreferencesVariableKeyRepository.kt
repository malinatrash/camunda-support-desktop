package com.malinatrash.camundasupport.data

import java.util.prefs.Preferences

class PreferencesVariableKeyRepository(
    private val root: Preferences = Preferences.userRoot().node("com/malinatrash/camunda-support/variable-keys"),
) : VariableKeyRepository {
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

    private companion object {
        const val KEYS = "keys"
    }
}
