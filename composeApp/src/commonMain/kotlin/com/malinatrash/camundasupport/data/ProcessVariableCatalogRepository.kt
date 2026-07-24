package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.ProcessVariableCatalog

data class StoredProcessVariableCatalog(
    val catalog: ProcessVariableCatalog,
    val loadedAtEpochMillis: Long,
)

interface ProcessVariableCatalogRepository {
    fun load(connectionRestUrl: String, processDefinitionKey: String): StoredProcessVariableCatalog?

    fun save(connectionRestUrl: String, stored: StoredProcessVariableCatalog)

    fun remove(connectionRestUrl: String, processDefinitionKey: String)
}

class InMemoryProcessVariableCatalogRepository : ProcessVariableCatalogRepository {
    private val catalogs = mutableMapOf<Pair<String, String>, StoredProcessVariableCatalog>()

    override fun load(
        connectionRestUrl: String,
        processDefinitionKey: String,
    ): StoredProcessVariableCatalog? = catalogs[connectionRestUrl to processDefinitionKey]

    override fun save(
        connectionRestUrl: String,
        stored: StoredProcessVariableCatalog,
    ) {
        catalogs[connectionRestUrl to stored.catalog.processDefinitionKey] = stored
    }

    override fun remove(connectionRestUrl: String, processDefinitionKey: String) {
        catalogs.remove(connectionRestUrl to processDefinitionKey)
    }
}
