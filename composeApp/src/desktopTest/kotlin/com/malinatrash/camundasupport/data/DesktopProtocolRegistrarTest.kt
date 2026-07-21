package com.malinatrash.camundasupport.data

import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class DesktopProtocolRegistrarTest {
    @Test
    fun `builds Windows protocol command with quoted launcher and URL`() {
        val commands = windowsProtocolRegistrationCommands("C:\\Program Files\\Camunda Support\\Camunda Support.exe")

        assertEquals(
            "\"C:\\Program Files\\Camunda Support\\Camunda Support.exe\" \"%1\"",
            commands.last()[5],
        )
        assertEquals("HKCU\\Software\\Classes\\camunda-support\\shell\\open\\command", commands.last()[2])
    }

    @Test
    fun `builds Linux scheme handler desktop entry`() {
        val entry = linuxProtocolDesktopEntry("/opt/camunda support/bin/Camunda Support")

        assertTrue("Exec=\"/opt/camunda support/bin/Camunda Support\" %u" in entry)
        assertTrue("MimeType=x-scheme-handler/camunda-support;" in entry)
    }

    @Test
    fun `registers Linux handler in user applications directory`() {
        val home = Files.createTempDirectory("camunda-support-home")
        val launcher = Files.createTempFile("Camunda Support", "")
        val commands = mutableListOf<List<String>>()
        try {
            DesktopProtocolRegistrar(
                osName = "Linux",
                launcherPath = launcher.toString(),
                userHome = home,
                commandRunner = { command -> commands += command; true },
            ).register()

            val desktopFile = home.resolve(
                ".local/share/applications/com.malinatrash.camundasupport-url.desktop",
            )
            assertTrue(Files.isRegularFile(desktopFile))
            assertTrue("x-scheme-handler/camunda-support" in Files.readString(desktopFile))
            assertEquals("xdg-mime", commands.single().first())
        } finally {
            home.toFile().deleteRecursively()
            Files.deleteIfExists(launcher)
        }
    }
}
