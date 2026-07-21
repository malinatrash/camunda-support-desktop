package com.malinatrash.camundasupport.model

const val APP_DEEP_LINK_SCHEME = "camunda-support"

data class ProcessInstanceDeepLink(
    val origin: String,
    val processInstanceId: String,
)

fun CamundaConnection.processInstanceDeepLink(processInstanceId: String): String? {
    if (!isValidProcessInstanceId(processInstanceId)) return null
    val origin = normalizedWebOrigin(cockpitUrl) ?: normalizedWebOrigin(restUrl) ?: return null
    return "$APP_DEEP_LINK_SCHEME://process-instance/${percentEncode(processInstanceId)}" +
        "?origin=${percentEncode(origin)}"
}

fun CamundaConnection.matchesOrigin(origin: String): Boolean {
    val normalizedOrigin = normalizedWebOrigin(origin) ?: return false
    return normalizedWebOrigin(cockpitUrl) == normalizedOrigin || normalizedWebOrigin(restUrl) == normalizedOrigin
}

fun normalizedWebOrigin(value: String): String? {
    val normalized = value.trim()
    val schemeSeparator = normalized.indexOf("://")
    if (schemeSeparator <= 0) return null
    val scheme = normalized.substring(0, schemeSeparator).lowercase()
    if (scheme != "http" && scheme != "https") return null

    val authorityStart = schemeSeparator + 3
    val authorityEnd = normalized.indexOfAny(charArrayOf('/', '?', '#'), authorityStart)
        .let { if (it == -1) normalized.length else it }
    if (authorityEnd <= authorityStart) return null
    var authority = normalized.substring(authorityStart, authorityEnd).lowercase()
    if (authority.isBlank() || authority.any(Char::isWhitespace) || '@' in authority) return null
    authority = when (scheme) {
        "http" -> authority.removeSuffix(":80")
        "https" -> authority.removeSuffix(":443")
        else -> authority
    }
    if (authority.isBlank()) return null
    return "$scheme://$authority"
}

fun isValidProcessInstanceId(value: String): Boolean =
    value.isNotBlank() &&
        value.length <= 256 &&
        value.none { it.isWhitespace() || it.isISOControl() || it == '/' || it == '?' || it == '#' }

private fun percentEncode(value: String): String = buildString {
    value.encodeToByteArray().forEach { byte ->
        val unsigned = byte.toInt() and 0xFF
        if (unsigned in 'A'.code..'Z'.code ||
            unsigned in 'a'.code..'z'.code ||
            unsigned in '0'.code..'9'.code ||
            unsigned == '-'.code || unsigned == '_'.code || unsigned == '.'.code || unsigned == '~'.code
        ) {
            append(unsigned.toChar())
        } else {
            append('%')
            append(HEX[unsigned ushr 4])
            append(HEX[unsigned and 0x0F])
        }
    }
}

private const val HEX = "0123456789ABCDEF"
