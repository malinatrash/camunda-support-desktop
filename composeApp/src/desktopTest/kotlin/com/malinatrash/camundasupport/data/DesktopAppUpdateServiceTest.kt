package com.malinatrash.camundasupport.data

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.net.http.HttpClient
import java.nio.file.Files
import java.nio.file.Path
import java.security.MessageDigest
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.runBlocking
import kotlin.io.path.createTempDirectory
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertIs

class DesktopAppUpdateServiceTest {
    @Test
    fun checksDownloadsVerifiesAndOpensPlatformInstaller() = runBlocking {
        val installerBytes = "проверенный dmg".encodeToByteArray()
        val installerName = "Camunda.Support-9.0.0.dmg"
        val checksumFileName = "Camunda Support-9.0.0.dmg"
        val checksum = installerBytes.sha256()
        val opened = AtomicReference<Path?>()
        val directory = createTempDirectory("camunda-update-test")
        val server = HttpServer.create(InetSocketAddress("localhost", 0), 0)
        server.start()
        val baseUrl = "http://localhost:${server.address.port}"
        server.createContext("/latest") { exchange ->
            exchange.respond(
                """
                    {
                      "tag_name": "v9.0.0",
                      "html_url": "$baseUrl/release",
                      "body": "Новая версия",
                      "assets": [
                        {
                          "name": "$installerName",
                          "browser_download_url": "$baseUrl/installer",
                          "size": ${installerBytes.size},
                          "digest": null
                        },
                        {
                          "name": "SHA256SUMS.txt",
                          "browser_download_url": "$baseUrl/checksums",
                          "size": 100,
                          "digest": null
                        }
                      ]
                    }
                """.trimIndent(),
            )
        }
        server.createContext("/installer") { exchange -> exchange.respond(installerBytes) }
        server.createContext("/checksums") { exchange ->
            exchange.respond("$checksum  ./$checksumFileName\n")
        }

        try {
            val service = DesktopAppUpdateService(
                client = HttpClient.newBuilder().followRedirects(HttpClient.Redirect.NORMAL).build(),
                apiUrl = "$baseUrl/latest",
                platform = DesktopUpdatePlatform.MacArm64,
                updatesDirectory = directory,
                openInstaller = opened::set,
            )

            val check = assertIs<UpdateCheckResult.Available>(service.checkForUpdate("1.0.0"))
            assertEquals(installerName, check.update.asset?.name)
            val install = assertIs<UpdateInstallResult.InstallerOpened>(service.downloadAndOpen(check.update))

            assertEquals(installerName, install.fileName)
            val openedFile = requireNotNull(opened.get())
            assertContentEquals(installerBytes, Files.readAllBytes(openedFile))
        } finally {
            server.stop(0)
            directory.toFile().deleteRecursively()
        }
    }

    private fun HttpExchange.respond(body: String) = respond(body.encodeToByteArray())

    private fun HttpExchange.respond(body: ByteArray) {
        sendResponseHeaders(200, body.size.toLong())
        responseBody.use { it.write(body) }
    }

    private fun ByteArray.sha256(): String = MessageDigest.getInstance("SHA-256")
        .digest(this)
        .joinToString("") { byte -> "%02x".format(byte) }
}
