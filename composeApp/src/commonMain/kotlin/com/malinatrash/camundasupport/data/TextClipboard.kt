package com.malinatrash.camundasupport.data

interface TextClipboard {
    fun copy(text: String): Boolean
}

object NoOpTextClipboard : TextClipboard {
    override fun copy(text: String): Boolean = false
}
