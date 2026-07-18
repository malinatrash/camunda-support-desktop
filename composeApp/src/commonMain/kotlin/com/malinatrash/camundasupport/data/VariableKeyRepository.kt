package com.malinatrash.camundasupport.data

interface VariableKeyRepository {
    fun load(connectionId: String): List<String>

    fun save(connectionId: String, key: String)
}

class InMemoryVariableKeyRepository : VariableKeyRepository {
    private val keys = mutableMapOf<String, MutableSet<String>>()

    override fun load(connectionId: String): List<String> = keys[connectionId]
        .orEmpty()
        .sortedBy(String::lowercase)

    override fun save(connectionId: String, key: String) {
        val normalized = key.trim()
        if (normalized.isNotEmpty()) keys.getOrPut(connectionId, ::mutableSetOf) += normalized
    }
}
