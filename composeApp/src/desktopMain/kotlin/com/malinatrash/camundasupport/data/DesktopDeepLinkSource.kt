package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.APP_DEEP_LINK_SCHEME
import com.malinatrash.camundasupport.model.ProcessInstanceDeepLink
import com.malinatrash.camundasupport.model.isValidProcessInstanceId
import com.malinatrash.camundasupport.model.normalizedWebOrigin
import java.awt.Desktop
import java.net.URI
import java.net.URLDecoder
import java.nio.charset.StandardCharsets
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.receiveAsFlow

class DesktopDeepLinkSource(initialArguments: List<String>) : DeepLinkSource {
    private val channel = Channel<DeepLinkEvent>(Channel.UNLIMITED)
    private var activationHandler: () -> Unit = {}

    override val events: Flow<DeepLinkEvent> = channel.receiveAsFlow()

    init {
        initialArguments.filter(::looksLikeSupportedLink).forEach(::open)
    }

    fun setActivationHandler(handler: () -> Unit) {
        activationHandler = handler
    }

    fun registerSystemHandler() {
        runCatching {
            if (!Desktop.isDesktopSupported()) return
            val desktop = Desktop.getDesktop()
            if (!desktop.isSupported(Desktop.Action.APP_OPEN_URI)) return
            desktop.setOpenURIHandler { event -> open(event.uri.toString()) }
        }
    }

    override fun open(rawValue: String) {
        channel.trySend(parseDesktopDeepLink(rawValue))
        activationHandler()
    }
}

internal fun parseDesktopDeepLink(rawValue: String): DeepLinkEvent {
    val value = rawValue.trim()
    val uri = runCatching { URI(value) }.getOrNull()
        ?: return DeepLinkEvent.Invalid("Ссылка имеет некорректный формат.")
    return when (uri.scheme?.lowercase()) {
        APP_DEEP_LINK_SCHEME -> parseApplicationLink(uri)
        "http", "https" -> parseCockpitLink(uri)
        else -> DeepLinkEvent.Invalid("Приложение не поддерживает протокол этой ссылки.")
    }
}

private fun parseApplicationLink(uri: URI): DeepLinkEvent {
    if (!uri.rawAuthority.equals("process-instance", ignoreCase = true)) {
        return DeepLinkEvent.Invalid("Ссылка не относится к заявке Camunda.")
    }
    val processInstanceId = decode(uri.rawPath.trim('/'))
        ?: return DeepLinkEvent.Invalid("В ссылке указан некорректный идентификатор заявки.")
    if (!isValidProcessInstanceId(processInstanceId)) {
        return DeepLinkEvent.Invalid("В ссылке указан некорректный идентификатор заявки.")
    }
    val origin = queryValue(uri.rawQuery, "origin")?.let(::normalizedWebOrigin)
        ?: return DeepLinkEvent.Invalid("В ссылке отсутствует корректный домен Camunda.")
    return DeepLinkEvent.OpenProcessInstance(ProcessInstanceDeepLink(origin, processInstanceId))
}

private fun parseCockpitLink(uri: URI): DeepLinkEvent {
    val origin = normalizedWebOrigin("${uri.scheme}://${uri.rawAuthority.orEmpty()}")
        ?: return DeepLinkEvent.Invalid("В ссылке отсутствует корректный домен Camunda.")
    val route = uri.rawFragment.orEmpty().substringBefore('?').trim('/')
    val segments = route.split('/')
    if (segments.size < 2 || segments[0] != "process-instance") {
        return DeepLinkEvent.Invalid("Ссылка не относится к заявке Camunda.")
    }
    val processInstanceId = decode(segments[1])
        ?: return DeepLinkEvent.Invalid("В ссылке указан некорректный идентификатор заявки.")
    if (!isValidProcessInstanceId(processInstanceId)) {
        return DeepLinkEvent.Invalid("В ссылке указан некорректный идентификатор заявки.")
    }
    return DeepLinkEvent.OpenProcessInstance(ProcessInstanceDeepLink(origin, processInstanceId))
}

private fun queryValue(rawQuery: String?, expectedName: String): String? = rawQuery
    ?.split('&')
    ?.asSequence()
    ?.map { parameter -> parameter.substringBefore('=') to parameter.substringAfter('=', "") }
    ?.firstOrNull { (name) -> decode(name) == expectedName }
    ?.second
    ?.let(::decode)

private fun decode(value: String): String? = runCatching {
    URLDecoder.decode(value, StandardCharsets.UTF_8)
}.getOrNull()

private fun looksLikeSupportedLink(value: String): Boolean {
    val normalized = value.trim().lowercase()
    return normalized.startsWith("$APP_DEEP_LINK_SCHEME:") ||
        ((normalized.startsWith("https://") || normalized.startsWith("http://")) &&
            "/camunda/app/cockpit/" in normalized)
}
