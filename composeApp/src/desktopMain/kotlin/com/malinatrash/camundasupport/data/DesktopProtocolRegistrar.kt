package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.APP_DEEP_LINK_SCHEME
import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.createDirectories
import kotlin.io.path.writeText

class DesktopProtocolRegistrar(
    private val osName: String = System.getProperty("os.name").orEmpty(),
    private val launcherPath: String = System.getProperty(LAUNCHER_PATH_PROPERTY).orEmpty(),
    private val userHome: Path = Path.of(System.getProperty("user.home").orEmpty()),
    private val commandRunner: (List<String>) -> Boolean = ::runCommand,
) {
    fun register() {
        if (!isPackagedLauncherPath(launcherPath)) return
        when {
            osName.contains("win", ignoreCase = true) -> registerWindowsProtocol()
            osName.contains("linux", ignoreCase = true) -> registerLinuxProtocol()
        }
    }

    private fun registerWindowsProtocol() {
        windowsProtocolRegistrationCommands(launcherPath).forEach { commandRunner(it) }
    }

    private fun registerLinuxProtocol() {
        if (userHome.toString().isBlank()) return
        val applicationsDirectory = userHome.resolve(".local/share/applications").createDirectories()
        val desktopFile = applicationsDirectory.resolve(LINUX_DESKTOP_FILE_NAME)
        runCatching { desktopFile.writeText(linuxProtocolDesktopEntry(launcherPath)) }.getOrElse { return }
        commandRunner(
            listOf(
                "xdg-mime",
                "default",
                LINUX_DESKTOP_FILE_NAME,
                "x-scheme-handler/$APP_DEEP_LINK_SCHEME",
            ),
        )
    }
}

internal fun windowsProtocolRegistrationCommands(launcherPath: String): List<List<String>> {
    val protocolKey = "HKCU\\Software\\Classes\\$APP_DEEP_LINK_SCHEME"
    return listOf(
        listOf("reg.exe", "add", protocolKey, "/ve", "/d", "URL:Camunda Support", "/f"),
        listOf("reg.exe", "add", protocolKey, "/v", "URL Protocol", "/d", "", "/f"),
        listOf(
            "reg.exe",
            "add",
            "$protocolKey\\DefaultIcon",
            "/ve",
            "/d",
            "\"$launcherPath\",0",
            "/f",
        ),
        listOf(
            "reg.exe",
            "add",
            "$protocolKey\\shell\\open\\command",
            "/ve",
            "/d",
            "\"$launcherPath\" \"%1\"",
            "/f",
        ),
    )
}

internal fun linuxProtocolDesktopEntry(launcherPath: String): String = """
    [Desktop Entry]
    Type=Application
    Name=Camunda Support
    Comment=Открытие заявок Camunda
    Exec=${desktopEntryQuote(launcherPath)} %u
    Terminal=false
    NoDisplay=true
    MimeType=x-scheme-handler/$APP_DEEP_LINK_SCHEME;
""".trimIndent() + "\n"

private fun desktopEntryQuote(value: String): String = "\"${value.replace("\\", "\\\\").replace("\"", "\\\"")}\""

private fun isPackagedLauncherPath(value: String): Boolean =
    value.isNotBlank() && '$' !in value && runCatching { Files.isRegularFile(Path.of(value)) }.getOrDefault(false)

private fun runCommand(command: List<String>): Boolean = runCatching {
    ProcessBuilder(command)
        .redirectErrorStream(true)
        .start()
        .apply { inputStream.bufferedReader().use { it.readText() } }
        .waitFor() == 0
}.getOrDefault(false)

private const val LAUNCHER_PATH_PROPERTY = "camunda.support.launcher.path"
private const val LINUX_DESKTOP_FILE_NAME = "com.malinatrash.camundasupport-url.desktop"
