package com.malinatrash.camundasupport.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.malinatrash.camundasupport.data.CamundaApi
import com.malinatrash.camundasupport.data.CamundaApiResult
import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.ProcessInstanceDetails
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlin.time.Duration.Companion.seconds
import kotlin.time.TimeMark
import kotlin.time.TimeSource

internal data class ProcessInstancePollingQuery(
    val connection: CamundaConnection,
    val processInstanceId: String,
)

internal class ProcessInstanceCacheEntry {
    var details by mutableStateOf<ProcessInstanceDetails?>(null)
        internal set

    var isRefreshing by mutableStateOf(false)
        internal set

    var error by mutableStateOf<String?>(null)
        internal set

    internal var lastSuccessfulRefresh: TimeMark? = null
}

/**
 * Общий для приложения кэш открытых активных заявок.
 *
 * Благодаря ему переключение между вкладками не показывает пустой экран, а повторный
 * сетевой запрос выполняется только когда снимок старше интервала автообновления.
 */
internal class ProcessInstancePollingStore(
    private val camundaApi: CamundaApi,
    private val timeSource: TimeSource = TimeSource.Monotonic,
) {
    private val entries = linkedMapOf<ProcessInstancePollingQuery, ProcessInstanceCacheEntry>()
    private val refreshMutex = Mutex()

    fun entry(query: ProcessInstancePollingQuery): ProcessInstanceCacheEntry =
        entries.getOrPut(query) {
            if (entries.size >= MAX_CACHED_INSTANCES) {
                entries.remove(entries.keys.first())
            }
            ProcessInstanceCacheEntry()
        }

    suspend fun refresh(
        query: ProcessInstancePollingQuery,
        force: Boolean = false,
    ) {
        val entry = entry(query)
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
            when (val result = camundaApi.loadProcessInstanceDetails(query.connection, query.processInstanceId)) {
                is CamundaApiResult.Success -> {
                    entry.details = result.value
                    entry.error = null
                    entry.lastSuccessfulRefresh = timeSource.markNow()
                }

                is CamundaApiResult.Failure -> entry.error = result.message
            }
        } catch (cancellation: CancellationException) {
            throw cancellation
        } catch (failure: Throwable) {
            entry.error = failure.message ?: "Не удалось обновить состояние заявки"
        } finally {
            refreshMutex.withLock { entry.isRefreshing = false }
        }
    }

    internal companion object {
        val POLLING_INTERVAL = 20.seconds
        const val MAX_CACHED_INSTANCES = 32
    }
}
