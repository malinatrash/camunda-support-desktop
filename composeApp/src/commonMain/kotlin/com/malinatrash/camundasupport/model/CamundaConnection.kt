package com.malinatrash.camundasupport.model

enum class Environment(
    val label: String,
    val shortLabel: String,
) {
    Production("Прод", "ПРОД"),
    Test("Тест", "ТЕСТ"),
    Development("Разработка", "РАЗР"),
    Local("Локальное", "ЛОКАЛ"),
}

enum class ConnectionHealth {
    Unknown,
    Checking,
    Connected,
    Failed,
}

data class CamundaConnection(
    val id: String,
    val name: String,
    val restUrl: String,
    val cockpitUrl: String,
    val environment: Environment,
    val health: ConnectionHealth = ConnectionHealth.Unknown,
    val engineVersion: String? = null,
)

data class ConnectionDraft(
    val name: String = "",
    val restUrl: String = "",
    val cockpitUrl: String = "",
    val environment: Environment = Environment.Test,
)

data class ConnectionValidation(
    val nameError: String? = null,
    val restUrlError: String? = null,
    val cockpitUrlError: String? = null,
) {
    val isValid: Boolean
        get() = nameError == null && restUrlError == null && cockpitUrlError == null
}

fun validateConnectionDraft(draft: ConnectionDraft): ConnectionValidation {
    fun genericUrlError(value: String, required: Boolean): String? {
        if (value.isBlank()) return if (required) "Укажите URL" else null
        val normalized = value.trim()
        if (!normalized.startsWith("https://") && !normalized.startsWith("http://")) {
            return "URL должен начинаться с https:// или http://"
        }
        if (normalized.any(Char::isWhitespace)) return "URL не должен содержать пробелы"
        if (normalized.contains('?') || normalized.contains('#')) return "URL не должен содержать query-параметры или fragment"
        val authorityAndPath = normalized.substringAfter("://")
        if (authorityAndPath.substringBefore('/').isBlank()) return "URL должен содержать адрес хоста"
        return null
    }

    fun restUrlError(value: String): String? {
        genericUrlError(value, required = true)?.let { return it }
        val normalized = value.trim().trimEnd('/')
        if (!normalized.endsWith("/engine-rest")) {
            return "Адрес Camunda REST должен заканчиваться на /engine-rest"
        }
        return null
    }

    return ConnectionValidation(
        nameError = if (draft.name.isBlank()) "Укажите название" else null,
        restUrlError = restUrlError(draft.restUrl),
        cockpitUrlError = genericUrlError(draft.cockpitUrl, required = false),
    )
}

fun ConnectionDraft.toConnection(
    id: String,
    engineVersion: String? = null,
): CamundaConnection = CamundaConnection(
    id = id,
    name = name.trim(),
    restUrl = restUrl.trim().trimEnd('/'),
    cockpitUrl = cockpitUrl.trim().trimEnd('/').ifBlank { deriveCockpitUrl(restUrl) },
    environment = environment,
    health = ConnectionHealth.Connected,
    engineVersion = engineVersion,
)

fun deriveCockpitUrl(restUrl: String): String {
    val normalized = restUrl.trim()
    val schemeSeparator = normalized.indexOf("://")
    if (schemeSeparator <= 0) return ""
    val authorityStart = schemeSeparator + 3
    val authorityEnd = normalized.indexOf('/', authorityStart).let { index ->
        if (index == -1) normalized.length else index
    }
    if (authorityEnd <= authorityStart) return ""
    return "${normalized.substring(0, authorityEnd)}/camunda/app/cockpit"
}
