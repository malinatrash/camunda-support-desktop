package com.malinatrash.camundasupport.data

import java.awt.Toolkit
import java.awt.datatransfer.StringSelection

class DesktopTextClipboard : TextClipboard {
    override fun copy(text: String): Boolean = runCatching {
        Toolkit.getDefaultToolkit().systemClipboard.setContents(StringSelection(text), null)
    }.isSuccess
}
