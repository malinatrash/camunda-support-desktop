package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.ProcessInstanceDeepLink
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow

sealed interface DeepLinkEvent {
    data class OpenProcessInstance(val link: ProcessInstanceDeepLink) : DeepLinkEvent

    data class Invalid(val message: String) : DeepLinkEvent
}

interface DeepLinkSource {
    val events: Flow<DeepLinkEvent>

    fun open(rawValue: String)
}

object NoOpDeepLinkSource : DeepLinkSource {
    override val events: Flow<DeepLinkEvent> = emptyFlow()

    override fun open(rawValue: String) = Unit
}
