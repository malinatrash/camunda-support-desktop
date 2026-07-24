package com.malinatrash.camundasupport.data

import com.malinatrash.camundasupport.model.CamundaConnection
import com.malinatrash.camundasupport.model.DeploymentSort
import com.malinatrash.camundasupport.model.DashboardDateFilter
import com.malinatrash.camundasupport.model.ProcessDashboard
import com.malinatrash.camundasupport.model.ProcessDefinitionDetails
import com.malinatrash.camundasupport.model.ProcessDefinitionSummary
import com.malinatrash.camundasupport.model.ProcessInstanceDetails
import com.malinatrash.camundasupport.model.ProcessInstanceSummary
import com.malinatrash.camundasupport.model.ProcessIncident
import com.malinatrash.camundasupport.model.ProcessVariableUpdate
import com.malinatrash.camundasupport.model.ProcessVariableCatalog
import com.malinatrash.camundasupport.model.TeleportRequest
import com.malinatrash.camundasupport.model.VariableValueType

sealed interface CamundaApiResult<out T> {
    data class Success<T>(val value: T) : CamundaApiResult<T>

    data class Failure(val message: String) : CamundaApiResult<Nothing>
}

interface CamundaApi {
    suspend fun loadDashboard(
        connection: CamundaConnection,
        sort: DeploymentSort,
        dateFilter: DashboardDateFilter,
    ): CamundaApiResult<ProcessDashboard>

    suspend fun searchProcessInstances(
        connection: CamundaConnection,
        processDefinitionKey: String,
        variableName: String,
        variableValue: String,
        valueType: VariableValueType,
    ): CamundaApiResult<List<ProcessInstanceSummary>>

    suspend fun loadSearchProcessDefinitions(
        connection: CamundaConnection,
    ): CamundaApiResult<List<ProcessDefinitionSummary>>

    suspend fun loadProcessVariableCatalog(
        connection: CamundaConnection,
        processDefinitionKey: String,
    ): CamundaApiResult<ProcessVariableCatalog>

    fun invalidateProcessVariableCatalog(
        connection: CamundaConnection,
        processDefinitionKey: String,
    ) = Unit

    suspend fun loadProcessDefinitionDetails(
        connection: CamundaConnection,
        processDefinitionId: String,
        dateFilter: DashboardDateFilter,
    ): CamundaApiResult<ProcessDefinitionDetails>

    suspend fun loadProcessInstanceDetails(
        connection: CamundaConnection,
        processInstanceId: String,
    ): CamundaApiResult<ProcessInstanceDetails>

    suspend fun teleportProcessInstance(
        connection: CamundaConnection,
        processInstanceId: String,
        request: TeleportRequest,
    ): CamundaApiResult<Unit>

    suspend fun setJobRetries(connection: CamundaConnection, jobId: String, retries: Int): CamundaApiResult<Unit>

    suspend fun setExternalTaskRetries(connection: CamundaConnection, externalTaskId: String, retries: Int): CamundaApiResult<Unit>

    suspend fun unlockExternalTask(connection: CamundaConnection, externalTaskId: String): CamundaApiResult<Unit>

    suspend fun setProcessInstanceSuspended(
        connection: CamundaConnection,
        processInstanceId: String,
        suspended: Boolean,
    ): CamundaApiResult<Unit>

    suspend fun updateProcessVariable(
        connection: CamundaConnection,
        processInstanceId: String,
        update: ProcessVariableUpdate,
    ): CamundaApiResult<Unit>

    suspend fun loadIncidents(
        connection: CamundaConnection,
        processDefinitionId: String?,
        dateFilter: DashboardDateFilter,
        processDefinitionKey: String? = null,
    ): CamundaApiResult<List<ProcessIncident>>

    suspend fun loadProcessDefinitionVersions(
        connection: CamundaConnection,
        processDefinitionKey: String,
    ): CamundaApiResult<List<ProcessDefinitionSummary>>
}
