package com.malinatrash.camundasupport.data

import java.io.IOException
import java.net.ConnectException
import java.net.URI
import java.net.UnknownHostException
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.net.http.HttpTimeoutException
import java.time.Duration
import javax.net.ssl.SSLException
import javax.net.ssl.SSLHandshakeException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class DesktopConnectionTester(
    private val client: HttpClient = HttpClient.newBuilder()
        .connectTimeout(CONNECT_TIMEOUT)
        .followRedirects(HttpClient.Redirect.NORMAL)
        .build(),
) : ConnectionTester {
    override suspend fun test(restUrl: String): ConnectionTestResult = withContext(Dispatchers.IO) {
        val versionUrl = "${restUrl.trim().trimEnd('/')}/version"
        try {
            val request = HttpRequest.newBuilder(URI(versionUrl))
                .timeout(REQUEST_TIMEOUT)
                .header("Accept", "application/json")
                .GET()
                .build()
            val response = client.send(request, HttpResponse.BodyHandlers.ofString())
            response.toConnectionTestResult(versionUrl)
        } catch (error: CancellationException) {
            throw error
        } catch (error: InterruptedException) {
            Thread.currentThread().interrupt()
            ConnectionTestResult.Failure("Проверка подключения была прервана. Повторите попытку.")
        } catch (error: Throwable) {
            error.toConnectionFailure(versionUrl)
        }
    }

    private fun HttpResponse<String>.toConnectionTestResult(versionUrl: String): ConnectionTestResult = when (statusCode()) {
        in 200..299 -> ConnectionTestResult.Success(extractVersion(body()))
        400 -> failure("Camunda отклонила проверочный запрос (HTTP 400). Проверьте REST URL.", versionUrl)
        401 -> failure("Требуется авторизация (HTTP 401). Подключение не имеет доступа к Camunda REST.", versionUrl)
        403 -> failure("Доступ запрещён (HTTP 403). У пользователя или сети нет доступа к Camunda REST.", versionUrl)
        404 -> failure("Endpoint Camunda не найден (HTTP 404). Проверьте, что REST API доступен по /engine-rest.", versionUrl)
        405 -> failure("Сервер запрещает GET /version (HTTP 405). Проверьте proxy и настройки Camunda REST.", versionUrl)
        408 -> failure("Camunda не успела обработать запрос (HTTP 408).", versionUrl)
        429 -> failure("Слишком много запросов (HTTP 429). Подождите и повторите попытку.", versionUrl)
        in 500..599 -> failure("Camunda или gateway вернули серверную ошибку HTTP ${statusCode()}.", versionUrl)
        in 300..399 -> failure("Неожиданный redirect HTTP ${statusCode()}. Проверьте REST URL и gateway авторизации.", versionUrl)
        else -> failure("Неожиданный HTTP-ответ: ${statusCode()}.", versionUrl)
    }

    private fun Throwable.toConnectionFailure(versionUrl: String): ConnectionTestResult {
        val causes = generateSequence(this) { it.cause }.toList()
        val message = when {
            causes.any { it is HttpTimeoutException } ->
                "Время ожидания подключения истекло. Проверьте URL, VPN и сеть."
            causes.any { it is UnknownHostException } ->
                "Хост не найден — ошибка DNS. Проверьте имя хоста и VPN."
            causes.any { it is SSLHandshakeException } ->
                "Не удалось установить TLS-соединение. Сертификат сервера может быть недействительным или недоверенным."
            causes.any { it is SSLException } ->
                "Ошибка защищённого соединения TLS. Проверьте сертификат и настройку HTTPS."
            causes.any { it is ConnectException } ->
                "Подключение отклонено или хост недоступен. Проверьте хост, порт, VPN и firewall."
            causes.any { it is IllegalArgumentException } ->
                "Указан некорректный REST URL."
            causes.any { it is IOException } ->
                "Сетевая ошибка: ${causes.first { it is IOException }.message.orEmpty().ifBlank { "запрос не выполнен" }}."
            else -> "Не удалось проверить подключение: ${this.message.orEmpty().ifBlank { this::class.simpleName.orEmpty() }}."
        }
        return failure(message, versionUrl)
    }

    private fun failure(message: String, versionUrl: String) = ConnectionTestResult.Failure(
        "$message\nПроверялся адрес: $versionUrl",
    )

    private fun extractVersion(body: String): String? = VERSION_PATTERN
        .find(body)
        ?.groupValues
        ?.getOrNull(1)

    private companion object {
        val CONNECT_TIMEOUT: Duration = Duration.ofSeconds(5)
        val REQUEST_TIMEOUT: Duration = Duration.ofSeconds(10)
        val VERSION_PATTERN = Regex("\\\"version\\\"\\s*:\\s*\\\"([^\\\"]+)\\\"")
    }
}
