package com.malinatrash.camundasupport.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.DeploymentSort
import com.malinatrash.camundasupport.model.ProcessDashboard
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeMark
import kotlin.time.TimeSource

internal data class DashboardPollingQuery(
    val connection: CamundaConnection,
    val dateFilter: DashboardDateFilter,
)

internal class DashboardCacheEntry {
    var dashboard by mutableStateOf<ProcessDashboard?>(null)
        internal set

    var isRefreshing by mutableStateOf(false)
        internal set

    var error by mutableStateOf<String?>(null)
        internal set

    internal var lastSuccessfulRefresh: TimeMark? = null
}

internal class DashboardPollingStore(
    private val camundaApi: CamundaApi,
    private val timeSource: TimeSource = TimeSource.Monotonic,
) {
    private val entries = mutableMapOf<DashboardPollingQuery, DashboardCacheEntry>()
    private val refreshMutex = Mutex()

    var activeQuery by mutableStateOf<DashboardPollingQuery?>(null)
        private set

    var secondsUntilRefresh by mutableIntStateOf(POLLING_INTERVAL.inWholeSeconds.toInt())
        private set

    fun updateCountdown(seconds: Int) {
        secondsUntilRefresh = seconds.coerceAtLeast(0)
    }

    fun selectConnection(connection: CamundaConnection?) {
        if (activeQuery?.connection?.id == connection?.id) return
        activeQuery = connection?.let { DashboardPollingQuery(it, DashboardDateFilter()) }
    }

    fun activate(
        connection: CamundaConnection?,
        dateFilter: DashboardDateFilter,
    ) {
        activeQuery = connection?.let { DashboardPollingQuery(it, dateFilter) }
    }

    fun entry(
        connection: CamundaConnection,
        dateFilter: DashboardDateFilter,
    ): DashboardCacheEntry = entries.getOrPut(DashboardPollingQuery(connection, dateFilter)) {
        DashboardCacheEntry()
    }

    suspend fun refreshActive(force: Boolean = false) {
        activeQuery?.let { refresh(it, force) }
    }

    suspend fun refresh(query: DashboardPollingQuery, force: Boolean = false) {
        val entry = entries.getOrPut(query) { DashboardCacheEntry() }
        val shouldRefresh = refreshMutex.withLock {
            val isFresh = entry.lastSuccessfulRefresh?.elapsedNow()?.let { it < POLLING_INTERVAL } == true
            if (entry.isRefreshing || (!force && isFresh)) {
                false
            } else {
                entry.isRefreshing = true
                true
            }
        }
        if (!shouldRefresh) return

        try {
            when (val result = camundaApi.loadDashboard(query.connection, DeploymentSort.NewestFirst, query.dateFilter)) {
                is CamundaApiResult.Success -> {
                    entry.dashboard = result.value
                    entry.error = null
                    entry.lastSuccessfulRefresh = timeSource.markNow()
                }

                is CamundaApiResult.Failure -> entry.error = result.message
            }
        } catch (cancellation: CancellationException) {
            throw cancellation
        } catch (failure: Throwable) {
            entry.error = failure.message ?: "Не удалось обновить дашборд"
        } finally {
            refreshMutex.withLock { entry.isRefreshing = false }
        }
    }

    internal companion object {
        val POLLING_INTERVAL = 10.seconds
    }
}
