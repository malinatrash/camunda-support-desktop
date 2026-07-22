package com.malinatrash.camundasupport.data

data class ReleaseDownloadStats(
    val totalDownloads: Long,
    val macOsDownloads: Long,
    val windowsDownloads: Long,
    val linuxDownloads: Long,
    val releaseCount: Int,
    val installerAssetCount: Int,
)

sealed interface ReleaseDownloadStatsResult {
    data class Success(val stats: ReleaseDownloadStats) : ReleaseDownloadStatsResult
    data class Failure(val message: String) : ReleaseDownloadStatsResult
}

interface ReleaseDownloadStatsService {
    suspend fun load(forceRefresh: Boolean = false): ReleaseDownloadStatsResult
}

object NoOpReleaseDownloadStatsService : ReleaseDownloadStatsService {
    override suspend fun load(forceRefresh: Boolean): ReleaseDownloadStatsResult =
        ReleaseDownloadStatsResult.Failure("Статистика скачиваний недоступна на этой платформе")
}
