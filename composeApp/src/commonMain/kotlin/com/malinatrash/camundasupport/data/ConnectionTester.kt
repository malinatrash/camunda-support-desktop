package com.malinatrash.camundasupport.data

sealed interface ConnectionTestResult {
    data class Success(
        val engineVersion: String?,
    ) : ConnectionTestResult

    data class Failure(
        val message: String,
    ) : ConnectionTestResult
}

interface ConnectionTester {
    suspend fun test(restUrl: String): ConnectionTestResult
}
