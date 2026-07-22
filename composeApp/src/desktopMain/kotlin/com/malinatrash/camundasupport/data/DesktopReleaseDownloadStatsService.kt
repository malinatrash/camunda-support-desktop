package com.malinatrash.camundasupport.data

import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull

class DesktopReleaseDownloadStatsService internal constructor(
    private val client: HttpClient,
    private val releasesApiUrl: String,
    private val nowMillis: () -> Long = System::currentTimeMillis,
    private val cacheTtlMillis: Long = CACHE_TTL_MILLIS,
) : ReleaseDownloadStatsService {
    constructor() : this(
        client = HttpClient.newBuilder()
            .connectTimeout(CONNECT_TIMEOUT)
            .followRedirects(HttpClient.Redirect.NORMAL)
            .build(),
        releasesApiUrl = RELEASES_API,
    )

    private val json = Json { ignoreUnknownKeys = true }
    private val loadMutex = Mutex()
    private var cached: CachedStats? = null

    override suspend fun load(forceRefresh: Boolean): ReleaseDownloadStatsResult = withContext(Dispatchers.IO) {
        loadMutex.withLock {
            val now = nowMillis()
            cached
                ?.takeIf { !forceRefresh && now - it.loadedAtMillis < cacheTtlMillis }
                ?.let { return@withLock ReleaseDownloadStatsResult.Success(it.stats) }

            runCatching { fetchAllPages() }
                .map { stats ->
                    cached = CachedStats(stats, nowMillis())
                    ReleaseDownloadStatsResult.Success(stats)
                }
                .getOrElse { failure ->
                    if (failure is CancellationException) throw failure
                    ReleaseDownloadStatsResult.Failure(
                        failure.message ?: "Не удалось получить статистику скачиваний",
                    )
                }
        }
    }

    private fun fetchAllPages(): ReleaseDownloadStats {
        var nextUrl: String? = releasesApiUrl
        val visitedUrls = mutableSetOf<String>()
        var pageCount = 0
        var releaseCount = 0
        var installerAssetCount = 0
        var macOsDownloads = 0L
        var windowsDownloads = 0L
        var linuxDownloads = 0L

        while (nextUrl != null) {
            check(pageCount++ < MAX_PAGES) { "GitHub вернул слишком много страниц релизов" }
            check(visitedUrls.add(nextUrl)) { "GitHub вернул циклическую пагинацию релизов" }
            val response = client.send(apiRequest(nextUrl), HttpResponse.BodyHandlers.ofString())
            check(response.statusCode() == 200) { githubError(response.statusCode()) }
            val releases = json.parseToJsonElement(response.body()).jsonArray
            releaseCount += releases.size

            releases.forEach { releaseValue ->
                releaseValue.jsonObject["assets"]?.jsonArray.orEmpty().forEach { assetValue ->
                    val asset = assetValue.jsonObject
                    val name = asset["name"]?.jsonPrimitive?.contentOrNull.orEmpty().lowercase()
                    val downloads = asset["download_count"]?.jsonPrimitive?.longOrNull ?: 0L
                    when {
                        name.endsWith(".dmg") -> macOsDownloads += downloads
                        name.endsWith(".exe") || name.endsWith(".msi") -> windowsDownloads += downloads
                        name.endsWith(".deb") -> linuxDownloads += downloads
                        else -> return@forEach
                    }
                    installerAssetCount += 1
                }
            }
            nextUrl = response.headers().firstValue("Link").orElse(null)?.nextPageUrl()
        }

        return ReleaseDownloadStats(
            totalDownloads = macOsDownloads + windowsDownloads + linuxDownloads,
            macOsDownloads = macOsDownloads,
            windowsDownloads = windowsDownloads,
            linuxDownloads = linuxDownloads,
            releaseCount = releaseCount,
            installerAssetCount = installerAssetCount,
        )
    }

    private fun apiRequest(url: String): HttpRequest = HttpRequest.newBuilder(URI(url))
        .timeout(REQUEST_TIMEOUT)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "camunda-support-desktop/$APP_VERSION")
        .GET()
        .build()

    private fun String.nextPageUrl(): String? = LINK_PART_PATTERN.findAll(this)
        .firstOrNull { it.groupValues[2] == "next" }
        ?.groupValues
        ?.get(1)

    private fun githubError(status: Int): String = when (status) {
        403, 429 -> "GitHub временно ограничил загрузку статистики"
        404 -> "GitHub не нашёл опубликованные релизы"
        else -> "GitHub вернул HTTP $status при загрузке статистики"
    }

    private data class CachedStats(
        val stats: ReleaseDownloadStats,
        val loadedAtMillis: Long,
    )

    internal companion object {
        const val RELEASES_API =
            "https://api.github.com/repos/malinatrash/camunda-support-desktop/releases?per_page=100"
        const val CACHE_TTL_MILLIS = 6 * 60 * 60 * 1_000L
        private const val MAX_PAGES = 100
        private val CONNECT_TIMEOUT = Duration.ofSeconds(10)
        private val REQUEST_TIMEOUT = Duration.ofSeconds(20)
        private val LINK_PART_PATTERN = Regex("<([^>]+)>;\\s*rel=\\\"([^\\\"]+)\\\"")
    }
}
