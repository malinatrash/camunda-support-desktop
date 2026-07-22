package com.malinatrash.camundasupport.data

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.net.http.HttpClient
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.runBlocking
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class DesktopReleaseDownloadStatsServiceTest {
    @Test
    fun sumsInstallersFromEveryReleasePageAndCachesResult() = runBlocking {
        val requestCount = AtomicInteger()
        val server = HttpServer.create(InetSocketAddress("localhost", 0), 0)
        server.start()
        val baseUrl = "http://localhost:${server.address.port}"
        server.createContext("/releases") { exchange ->
            requestCount.incrementAndGet()
            exchange.responseHeaders.add("Link", "<$baseUrl/releases-page-2>; rel=\"next\"")
            exchange.respond(
                """
                    [
                      {"assets":[
                        {"name":"Camunda Support-1.0.0.dmg","download_count":2},
                        {"name":"Camunda Support-1.0.0.exe","download_count":3},
                        {"name":"SHA256SUMS.txt","download_count":99}
                      ]}
                    ]
                """.trimIndent(),
            )
        }
        server.createContext("/releases-page-2") { exchange ->
            requestCount.incrementAndGet()
            exchange.respond(
                """
                    [
                      {"assets":[
                        {"name":"Camunda Support-0.9.0.msi","download_count":4},
                        {"name":"Camunda Support-0.9.0.deb","download_count":5},
                        {"name":"source.zip","download_count":50}
                      ]}
                    ]
                """.trimIndent(),
            )
        }

        try {
            val service = DesktopReleaseDownloadStatsService(
                client = HttpClient.newHttpClient(),
                releasesApiUrl = "$baseUrl/releases",
                nowMillis = { 1_000L },
            )

            val first = assertIs<ReleaseDownloadStatsResult.Success>(service.load()).stats
            assertEquals(14, first.totalDownloads)
            assertEquals(2, first.macOsDownloads)
            assertEquals(7, first.windowsDownloads)
            assertEquals(5, first.linuxDownloads)
            assertEquals(2, first.releaseCount)
            assertEquals(4, first.installerAssetCount)
            assertEquals(2, requestCount.get())

            assertEquals(first, assertIs<ReleaseDownloadStatsResult.Success>(service.load()).stats)
            assertEquals(2, requestCount.get(), "Повторная загрузка должна использовать шестичасовой кэш")

            assertEquals(first, assertIs<ReleaseDownloadStatsResult.Success>(service.load(forceRefresh = true)).stats)
            assertEquals(4, requestCount.get(), "Ручное обновление должно заново пройти все страницы")
        } finally {
            server.stop(0)
        }
    }

    private fun HttpExchange.respond(body: String) {
        val bytes = body.encodeToByteArray()
        sendResponseHeaders(200, bytes.size.toLong())
        responseBody.use { it.write(bytes) }
    }
}
