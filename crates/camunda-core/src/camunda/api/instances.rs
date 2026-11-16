use serde_json::Value;

use super::ApiResult;
use super::json::{boolean, json_array, string};
use crate::camunda::http::ApiError;
use crate::models::*;

pub(super) fn parse_instances(body: &str) -> ApiResult<Vec<ProcessInstanceSummary>> {
    Ok(json_array(body)?.iter().filter_map(|value| parse_instance(value).ok()).collect())
}

pub(super) fn parse_instance(value: &Value) -> ApiResult<ProcessInstanceSummary> {
    let id = string(value, "id").ok_or_else(|| ApiError::Invalid(rust_i18n::t!("core.json.missing_instance_id").to_string()))?;
    let definition_id = string(value, "definitionId")
        .or_else(|| string(value, "processDefinitionId"))
        .ok_or_else(|| ApiError::Invalid(rust_i18n::t!("core.json.missing_definition_id").to_string()))?;
    let state = string(value, "state");
    let end_time = string(value, "endTime");
    let ended_by_state = state.as_deref().is_some_and(|state| ENDED_HISTORY_STATES.contains(&state));
    Ok(ProcessInstanceSummary {
        definition_key: string(value, "definitionKey")
            .or_else(|| string(value, "processDefinitionKey"))
            .unwrap_or_else(|| definition_id.split(':').next().unwrap_or_default().to_string()),
        id,
        definition_id,
        business_key: string(value, "businessKey"),
        suspended: boolean(value, "suspended") || state.as_deref() == Some("SUSPENDED"),
        tenant_id: string(value, "tenantId"),
        ended: boolean(value, "ended") || end_time.is_some() || ended_by_state,
        start_time: string(value, "startTime"),
        end_time,
        state,
        definition_name: string(value, "processDefinitionName"),
    })
}

const ENDED_HISTORY_STATES: &[&str] = &["COMPLETED", "EXTERNALLY_TERMINATED", "INTERNALLY_TERMINATED"];
