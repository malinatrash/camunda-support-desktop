package com.malinatrash.camundasupport.data

import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import java.security.MessageDigest
import java.time.Duration
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

internal enum class DesktopUpdatePlatform {
    MacArm64,
    Windows,
    Linux,
    Unsupported,
}

class DesktopAppUpdateService internal constructor(
    private val client: HttpClient,
    private val apiUrl: String,
    private val platform: DesktopUpdatePlatform,
    private val updatesDirectory: Path,
    private val openInstaller: (Path) -> Unit,
) : AppUpdateService {
    constructor() : this(
        client = HttpClient.newBuilder()
            .connectTimeout(CONNECT_TIMEOUT)
            .followRedirects(HttpClient.Redirect.NORMAL)
            .build(),
        apiUrl = LATEST_RELEASE_API,
        platform = detectPlatform(),
        updatesDirectory = Path.of(System.getProperty("java.io.tmpdir"), "camunda-support-updates"),
        openInstaller = { installer ->
            when (detectPlatform()) {
                DesktopUpdatePlatform.MacArm64 -> ProcessBuilder("open", installer.toString()).start()
                DesktopUpdatePlatform.Windows -> ProcessBuilder(installer.toString()).start()
                DesktopUpdatePlatform.Linux -> ProcessBuilder("xdg-open", installer.toString()).start()
                DesktopUpdatePlatform.Unsupported -> error("Автообновление недоступно на этой платформе")
            }
        },
    )

    private val json = Json { ignoreUnknownKeys = true }

    override suspend fun checkForUpdate(currentVersion: String): UpdateCheckResult = withContext(Dispatchers.IO) {
        runCatching {
            val response = client.send(apiRequest(apiUrl), HttpResponse.BodyHandlers.ofString())
            check(response.statusCode() == 200) { githubError(response.statusCode()) }
            val release = json.parseToJsonElement(response.body()).jsonObject
            val tag = release.requiredString("tag_name")
            if (!isNewerVersion(tag, currentVersion)) return@runCatching UpdateCheckResult.UpToDate

            val assets = release["assets"]?.jsonArray.orEmpty().map { value ->
                val asset = value.jsonObject
                AppUpdateAsset(
                    name = asset.requiredString("name"),
                    downloadUrl = asset.requiredString("browser_download_url"),
                    sizeBytes = asset["size"]?.jsonPrimitive?.contentOrNull?.toLongOrNull() ?: 0L,
                    digest = asset["digest"]?.jsonPrimitive?.contentOrNull,
                )
            }
            val installer = selectInstaller(assets, platform)
            val checksumUrl = assets
                .firstOrNull { it.name.equals("SHA256SUMS.txt", ignoreCase = true) }
                ?.downloadUrl

            UpdateCheckResult.Available(
                AppUpdate(
                    version = tag.removePrefix("v"),
                    tag = tag,
                    releaseUrl = release.requiredString("html_url"),
                    notes = release["body"]?.jsonPrimitive?.contentOrNull.orEmpty(),
                    asset = installer,
                    checksumUrl = checksumUrl,
                ),
            )
        }.getOrElse { failure ->
            if (failure is CancellationException) throw failure
            UpdateCheckResult.Failure(failure.message ?: "Не удалось проверить обновления")
        }
    }

    override suspend fun downloadAndOpen(
        update: AppUpdate,
        onProgress: (UpdateDownloadProgress) -> Unit,
    ): UpdateInstallResult = withContext(Dispatchers.IO) {
        val asset = update.asset
            ?: return@withContext UpdateInstallResult.Failure("Для этой платформы установщик не опубликован")
        runCatching {
            require(URI(asset.downloadUrl).scheme == "https" || apiUrl.startsWith("http://localhost")) {
                "GitHub вернул небезопасную ссылку на установщик"
            }
            val releaseDirectory = updatesDirectory.resolve(update.tag.safeFileName())
            Files.createDirectories(releaseDirectory)
            val target = releaseDirectory.resolve(Path.of(asset.name).fileName.toString())
            val destinationPath = target.toString()
            onProgress(UpdateDownloadProgress(UpdateDownloadStage.Preparing, 0, 0, 0, destinationPath))
            val expectedDigest = expectedDigest(update, asset)
            val cachedInstallerValid = if (Files.exists(target)) {
                onProgress(UpdateDownloadProgress(UpdateDownloadStage.Verifying, 0, 0, 0, destinationPath))
                target.sha256() == expectedDigest
            } else {
                false
            }
            if (!cachedInstallerValid) {
                val partial = target.resolveSibling("${target.fileName}.part")
                Files.deleteIfExists(partial)
                val response = client.send(downloadRequest(asset.downloadUrl), HttpResponse.BodyHandlers.ofInputStream())
                if (response.statusCode() !in 200..299) {
                    response.body().close()
                    error("GitHub не отдал установщик: HTTP ${response.statusCode()}")
                }
                val totalBytes = response.headers().firstValueAsLong("Content-Length").orElse(asset.sizeBytes)
                    .takeIf { it > 0 }
                    ?: asset.sizeBytes
                val actualDigest = response.body().use { input ->
                    val digest = MessageDigest.getInstance("SHA-256")
                    val startedAt = System.nanoTime()
                    var lastReportedAt = 0L
                    var downloadedBytes = 0L
                    onProgress(UpdateDownloadProgress(UpdateDownloadStage.Downloading, 0, totalBytes, 0, destinationPath))
                    Files.newOutputStream(partial).buffered().use { output ->
                        val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                        while (true) {
                            val read = input.read(buffer)
                            if (read < 0) break
                            output.write(buffer, 0, read)
                            digest.update(buffer, 0, read)
                            downloadedBytes += read
                            val now = System.nanoTime()
                            if (now - lastReportedAt >= PROGRESS_INTERVAL_NANOS) {
                                onProgress(
                                    UpdateDownloadProgress(
                                        stage = UpdateDownloadStage.Downloading,
                                        downloadedBytes = downloadedBytes,
                                        totalBytes = totalBytes,
                                        bytesPerSecond = averageSpeed(downloadedBytes, now - startedAt),
                                        destinationPath = destinationPath,
                                    ),
                                )
                                lastReportedAt = now
                            }
                        }
                    }
                    onProgress(
                        UpdateDownloadProgress(
                            stage = UpdateDownloadStage.Downloading,
                            downloadedBytes = downloadedBytes,
                            totalBytes = totalBytes,
                            bytesPerSecond = averageSpeed(downloadedBytes, System.nanoTime() - startedAt),
                            destinationPath = destinationPath,
                        ),
                    )
                    digest.digest().joinToString("") { byte -> "%02x".format(byte) }
                }
                onProgress(
                    UpdateDownloadProgress(
                        UpdateDownloadStage.Verifying,
                        totalBytes,
                        totalBytes,
                        destinationPath = destinationPath,
                    ),
                )
                check(actualDigest == expectedDigest) { "Контрольная сумма установщика не совпала" }
                Files.move(partial, target, StandardCopyOption.REPLACE_EXISTING)
            }
            onProgress(
                UpdateDownloadProgress(
                    UpdateDownloadStage.OpeningInstaller,
                    asset.sizeBytes,
                    asset.sizeBytes,
                    destinationPath = destinationPath,
                ),
            )
            openInstaller(target)
            UpdateInstallResult.InstallerOpened(asset.name)
        }.getOrElse { failure ->
            if (failure is CancellationException) throw failure
            UpdateInstallResult.Failure(failure.message ?: "Не удалось скачать обновление")
        }
    }

    private fun expectedDigest(update: AppUpdate, asset: AppUpdateAsset): String {
        asset.digest
            ?.takeIf { it.startsWith("sha256:", ignoreCase = true) }
            ?.substringAfter(':')
            ?.lowercase()
            ?.takeIf { it.length == SHA256_LENGTH }
            ?.let { return it }

        val checksumUrl = update.checksumUrl ?: error("В релизе отсутствует SHA256SUMS.txt")
        val response = client.send(downloadRequest(checksumUrl), HttpResponse.BodyHandlers.ofString())
        check(response.statusCode() in 200..299) { "Не удалось получить контрольные суммы обновления" }
        return response.body().lineSequence().mapNotNull { line ->
            val hash = line.take(SHA256_LENGTH).lowercase()
            val fileName = line.drop(SHA256_LENGTH).trim().removePrefix("*").removePrefix("./")
            hash.takeIf {
                it.length == SHA256_LENGTH && fileName.releaseAssetName() == asset.name.releaseAssetName()
            }
        }.firstOrNull() ?: error("Для ${asset.name} не найдена контрольная сумма")
    }

    private fun apiRequest(url: String): HttpRequest = HttpRequest.newBuilder(URI(url))
        .timeout(REQUEST_TIMEOUT)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "camunda-support-desktop/$APP_VERSION")
        .GET()
        .build()

    private fun downloadRequest(url: String): HttpRequest = HttpRequest.newBuilder(URI(url))
        .timeout(DOWNLOAD_TIMEOUT)
        .header("User-Agent", "camunda-support-desktop/$APP_VERSION")
        .GET()
        .build()

    private fun Path.sha256(): String {
        val digest = MessageDigest.getInstance("SHA-256")
        Files.newInputStream(this).buffered().use { input ->
            val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        return digest.digest().joinToString("") { byte -> "%02x".format(byte) }
    }

    private fun String.safeFileName(): String = map { char ->
        if (char.isLetterOrDigit() || char in setOf('.', '-', '_')) char else '_'
    }.joinToString("")

    private fun String.releaseAssetName(): String = replace(' ', '.')

    private fun kotlinx.serialization.json.JsonObject.requiredString(key: String): String =
        this[key]?.jsonPrimitive?.contentOrNull?.takeIf(String::isNotBlank)
            ?: error("GitHub не вернул поле $key")

    private fun githubError(status: Int): String = when (status) {
        403, 429 -> "GitHub временно ограничил проверку обновлений"
        404 -> "В GitHub пока нет опубликованного релиза"
        else -> "GitHub вернул HTTP $status"
    }

    internal companion object {
        const val LATEST_RELEASE_API =
            "https://api.github.com/repos/malinatrash/camunda-support-desktop/releases/latest"
        private const val SHA256_LENGTH = 64
        private const val PROGRESS_INTERVAL_NANOS = 100_000_000L
        private val CONNECT_TIMEOUT = Duration.ofSeconds(10)
        private val REQUEST_TIMEOUT = Duration.ofSeconds(20)
        private val DOWNLOAD_TIMEOUT = Duration.ofMinutes(10)

        private fun averageSpeed(downloadedBytes: Long, elapsedNanos: Long): Long =
            if (elapsedNanos <= 0) 0
            else (downloadedBytes.toDouble() * 1_000_000_000.0 / elapsedNanos.toDouble()).toLong()

        fun detectPlatform(): DesktopUpdatePlatform {
            val os = System.getProperty("os.name").lowercase()
            val arch = System.getProperty("os.arch").lowercase()
            return when {
                os.contains("mac") && (arch.contains("aarch64") || arch.contains("arm64")) ->
                    DesktopUpdatePlatform.MacArm64
                os.contains("win") -> DesktopUpdatePlatform.Windows
                os.contains("linux") -> DesktopUpdatePlatform.Linux
                else -> DesktopUpdatePlatform.Unsupported
            }
        }

        fun selectInstaller(
            assets: List<AppUpdateAsset>,
            platform: DesktopUpdatePlatform,
        ): AppUpdateAsset? = when (platform) {
            DesktopUpdatePlatform.MacArm64 -> assets.firstOrNull { it.name.endsWith(".dmg", ignoreCase = true) }
            DesktopUpdatePlatform.Windows -> assets.firstOrNull { it.name.endsWith(".exe", ignoreCase = true) }
                ?: assets.firstOrNull { it.name.endsWith(".msi", ignoreCase = true) }
            DesktopUpdatePlatform.Linux -> assets.firstOrNull { it.name.endsWith(".deb", ignoreCase = true) }
            DesktopUpdatePlatform.Unsupported -> null
        }
    }
}
