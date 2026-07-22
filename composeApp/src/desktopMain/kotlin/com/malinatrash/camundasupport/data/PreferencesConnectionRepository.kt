package com.malinatrash.camundasupport.data

import java.util.prefs.Preferences
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.model.deriveCockpitUrl

class PreferencesConnectionRepository(
    private val root: Preferences = Preferences.userRoot().node("com/malinatrash/camunda-support/connections"),
) : ConnectionRepository {
    override fun load(): List<CamundaConnection> = root.childrenNames().mapNotNull { connectionId ->
        val node = root.node(connectionId)
        val name = node.get(KEY_NAME, "").trim()
        val restUrl = node.get(KEY_REST_URL, "").trim()
        if (name.isBlank() || restUrl.isBlank()) return@mapNotNull null

        val storedCockpitUrl = node.get(KEY_COCKPIT_URL, "").trim()
        val cockpitUrl = storedCockpitUrl.ifBlank { deriveCockpitUrl(restUrl) }
        if (storedCockpitUrl.isBlank() && cockpitUrl.isNotBlank()) {
            node.put(KEY_COCKPIT_URL, cockpitUrl)
            node.flush()
        }

        CamundaConnection(
            id = connectionId,
            name = name,
            restUrl = restUrl,
            cockpitUrl = cockpitUrl,
            environment = node.get(KEY_ENVIRONMENT, Environment.Test.name)
                .let { value -> runCatching { Environment.valueOf(value) }.getOrDefault(Environment.Test) },
            engineVersion = node.get(KEY_ENGINE_VERSION, "").ifBlank { null },
        )
    }

    override fun save(connection: CamundaConnection) {
        root.node(connection.id).apply {
            put(KEY_NAME, connection.name)
            put(KEY_REST_URL, connection.restUrl)
            put(KEY_COCKPIT_URL, connection.cockpitUrl)
            put(KEY_ENVIRONMENT, connection.environment.name)
            connection.engineVersion?.let { put(KEY_ENGINE_VERSION, it) } ?: remove(KEY_ENGINE_VERSION)
            flush()
        }
    }

    override fun delete(connectionId: String) {
        root.node(connectionId).removeNode()
        root.flush()
    }

    private companion object {
        const val KEY_NAME = "name"
        const val KEY_REST_URL = "restUrl"
        const val KEY_COCKPIT_URL = "cockpitUrl"
        const val KEY_ENVIRONMENT = "environment"
        const val KEY_ENGINE_VERSION = "engineVersion"
    }
}
