package com.malinatrash.camundasupport.data

import java.awt.Desktop
import java.net.URI

class DesktopExternalNavigator : ExternalNavigator {
    override fun open(url: String) {
        val uri = runCatching { URI(url) }.getOrNull() ?: return
        if (uri.scheme !in setOf("https", "http")) return
        if (!Desktop.isDesktopSupported()) return
        Desktop.getDesktop().browse(uri)
    }
}
