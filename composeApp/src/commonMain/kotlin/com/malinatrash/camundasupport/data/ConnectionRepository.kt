package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.CamundaConnection

interface ConnectionRepository {
    fun load(): List<CamundaConnection>

    fun save(connection: CamundaConnection)

    fun delete(connectionId: String)
}
class InMemoryConnectionRepository(
    initialConnections: List<CamundaConnection> = emptyList(),
) : ConnectionRepository {
    private val connections = initialConnections.associateByTo(mutableMapOf()) { it.id }

    override fun load(): List<CamundaConnection> = connections.values.toList()

    override fun save(connection: CamundaConnection) {
        connections[connection.id] = connection
    }

    override fun delete(connectionId: String) {
        connections.remove(connectionId)
    }
}
