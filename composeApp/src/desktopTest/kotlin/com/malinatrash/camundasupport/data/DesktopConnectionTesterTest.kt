package com.malinatrash.camundasupport.data

import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import kotlinx.coroutines.runBlocking
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertIs

class DesktopConnectionTesterTest {
    private lateinit var server: HttpServer

    @BeforeTest
    fun setUp() {
        server = HttpServer.create(InetSocketAddress("127.0.0.1", 0), 0)
    }

    @AfterTest
    fun tearDown() {
        server.stop(0)
    }

    @Test
    fun successfulPingReturnsEngineVersion() = runBlocking {
        startServer(status = 200, body = """{"version":"7.21.0"}""")

        val result = DesktopConnectionTester().test(restUrl())

        assertIs<ConnectionTestResult.Success>(result)
        assertEquals("7.21.0", result.engineVersion)
    }

    @Test
    fun unauthorizedPingReturnsSpecificError() = runBlocking {
        startServer(status = 401, body = "Unauthorized")

        val result = DesktopConnectionTester().test(restUrl())

        assertIs<ConnectionTestResult.Failure>(result)
        assertContains(result.message, "Требуется авторизация (HTTP 401)")
        assertContains(result.message, "/engine-rest/version")
    }

    @Test
    fun missingEndpointReturnsSpecificError() = runBlocking {
        startServer(status = 404, body = "Not found")

        val result = DesktopConnectionTester().test(restUrl())

        assertIs<ConnectionTestResult.Failure>(result)
        assertContains(result.message, "Endpoint Camunda не найден (HTTP 404)")
    }

    private fun startServer(status: Int, body: String) {
        server.createContext("/engine-rest/version") { exchange ->
            val bytes = body.encodeToByteArray()
            exchange.responseHeaders.add("Content-Type", "application/json")
            exchange.sendResponseHeaders(status, bytes.size.toLong())
            exchange.responseBody.use { it.write(bytes) }
        }
        server.start()
    }

    private fun restUrl(): String = "http://127.0.0.1:${server.address.port}/engine-rest"
}
