package com.malinatrash.camundasupport.data

expect val APP_VERSION: String
expect val APP_BUILD: String

data class AppUpdateAsset(
    val name: String,
    val downloadUrl: String,
    val sizeBytes: Long,
    val digest: String?,
)

data class AppUpdate(
    val version: String,
    val tag: String,
    val releaseUrl: String,
    val notes: String,
    val asset: AppUpdateAsset?,
    val checksumUrl: String?,
)

sealed interface UpdateCheckResult {
    data object UpToDate : UpdateCheckResult
    data class Available(val update: AppUpdate) : UpdateCheckResult
    data class Failure(val message: String) : UpdateCheckResult
}

sealed interface UpdateInstallResult {
    data class InstallerOpened(val fileName: String) : UpdateInstallResult
    data class Failure(val message: String) : UpdateInstallResult
}

interface AppUpdateService {
    suspend fun checkForUpdate(currentVersion: String = APP_VERSION): UpdateCheckResult

    suspend fun downloadAndOpen(update: AppUpdate): UpdateInstallResult
}

object NoOpAppUpdateService : AppUpdateService {
    override suspend fun checkForUpdate(currentVersion: String): UpdateCheckResult = UpdateCheckResult.UpToDate

    override suspend fun downloadAndOpen(update: AppUpdate): UpdateInstallResult =
        UpdateInstallResult.Failure("Автообновление недоступно на этой платформе")
}

internal fun isNewerVersion(candidate: String, current: String): Boolean {
    fun parts(value: String): List<Int> = value
        .trim()
        .removePrefix("v")
        .substringBefore('-')
        .split('.')
        .map { it.toIntOrNull() ?: 0 }

    val candidateParts = parts(candidate)
    val currentParts = parts(current)
    val size = maxOf(candidateParts.size, currentParts.size, 3)
    return (0 until size)
        .map { index -> candidateParts.getOrElse(index) { 0 }.compareTo(currentParts.getOrElse(index) { 0 }) }
        .firstOrNull { it != 0 }
        ?.let { it > 0 }
        ?: false
}
