package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.ProcessVariableCatalog
import com.malinatrash.camundasupport.model.ProcessVariableDescriptor
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.prefs.Preferences

class PreferencesProcessVariableCatalogRepository(
    private val root: Preferences = Preferences.userRoot()
        .node("com/malinatrash/camunda-support/process-variable-catalogs"),
) : ProcessVariableCatalogRepository {
    override fun load(
        connectionRestUrl: String,
        processDefinitionKey: String,
    ): StoredProcessVariableCatalog? = runCatching {
        val key = cacheKey(connectionRestUrl, processDefinitionKey)
        if (key !in root.childrenNames()) return null
        val node = root.node(key)
        if (node.get(REST_URL, null) != connectionRestUrl || node.get(PROCESS_KEY, null) != processDefinitionKey) {
            return null
        }
        val loadedAt = node.getLong(LOADED_AT, -1L)
        val count = node.getInt(VARIABLE_COUNT, -1)
        if (loadedAt < 0 || count < 0) return null
        val variablesNode = node.node(VARIABLES)
        val variables = (0 until count).mapNotNull { index ->
            val variableNode = variablesNode.node(index.toString())
            val name = variableNode.get(NAME, "").trim()
            if (name.isEmpty()) return@mapNotNull null
            ProcessVariableDescriptor(
                name = name,
                types = variableNode.get(TYPES, "")
                    .lineSequence()
                    .map(String::trim)
                    .filter(String::isNotEmpty)
                    .distinct()
                    .toList(),
                occurrences = variableNode.getInt(OCCURRENCES, 0),
            )
        }
        StoredProcessVariableCatalog(
            catalog = ProcessVariableCatalog(
                processDefinitionKey = processDefinitionKey,
                variables = variables,
                inspectedInstanceCount = node.getInt(INSPECTED_INSTANCES, 0),
                instancesTruncated = node.getBoolean(INSTANCES_TRUNCATED, false),
            ),
            loadedAtEpochMillis = loadedAt,
        )
    }.getOrNull()

    override fun save(
        connectionRestUrl: String,
        stored: StoredProcessVariableCatalog,
    ) {
        runCatching {
            val catalog = stored.catalog
            val node = root.node(cacheKey(connectionRestUrl, catalog.processDefinitionKey))
            node.put(REST_URL, connectionRestUrl)
            node.put(PROCESS_KEY, catalog.processDefinitionKey)
            node.putLong(LOADED_AT, stored.loadedAtEpochMillis)
            node.putInt(INSPECTED_INSTANCES, catalog.inspectedInstanceCount)
            node.putBoolean(INSTANCES_TRUNCATED, catalog.instancesTruncated)
            node.putInt(VARIABLE_COUNT, catalog.variables.size)
            val variablesNode = node.node(VARIABLES)
            variablesNode.childrenNames().forEach { variablesNode.node(it).removeNode() }
            catalog.variables.forEachIndexed { index, variable ->
                variablesNode.node(index.toString()).apply {
                    put(NAME, variable.name)
                    put(TYPES, variable.types.joinToString("\n"))
                    putInt(OCCURRENCES, variable.occurrences)
                }
            }
            node.flush()
        }
    }

    override fun remove(connectionRestUrl: String, processDefinitionKey: String) {
        runCatching {
            val key = cacheKey(connectionRestUrl, processDefinitionKey)
            if (key in root.childrenNames()) {
                root.node(key).removeNode()
                root.flush()
            }
        }
    }

    private fun cacheKey(connectionRestUrl: String, processDefinitionKey: String): String {
        val bytes = "$connectionRestUrl\n$processDefinitionKey".toByteArray(StandardCharsets.UTF_8)
        return MessageDigest.getInstance("SHA-256")
            .digest(bytes)
            .joinToString("") { byte -> "%02x".format(byte) }
    }

    private companion object {
        const val REST_URL = "rest-url"
        const val PROCESS_KEY = "process-key"
        const val LOADED_AT = "loaded-at"
        const val INSPECTED_INSTANCES = "inspected-instances"
        const val INSTANCES_TRUNCATED = "instances-truncated"
        const val VARIABLE_COUNT = "variable-count"
        const val VARIABLES = "variables"
        const val NAME = "name"
        const val TYPES = "types"
        const val OCCURRENCES = "occurrences"
    }
}
