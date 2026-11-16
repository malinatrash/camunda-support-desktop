use std::collections::HashSet;

use serde_json::Value;

use super::json::{boolean, int, json_array, nullable_boolean, nullable_int, string};
use super::{ApiResult, CamundaApi};
use crate::camunda::http::{self, ApiError, encode};
use crate::models::*;

impl CamundaApi {
    pub async fn load_process_definition_versions(
        &self,
        connection: &CamundaConnection,
        process_definition_key: &str,
    ) -> Result<Vec<ProcessDefinitionSummary>, String> {
        let url =
            format!("{}/process-definition?key={}&sortBy=version&sortOrder=desc", connection.rest_url, encode(process_definition_key));
        let result = async { parse_definitions(&self.get(&url).await?) }.await;
        result.map_err(|error| http::user_message(&error, &url))
    }

    pub async fn load_search_process_definitions(&self, connection: &CamundaConnection) -> Result<Vec<ProcessDefinitionSummary>, String> {
        let url = format!(
            "{}/process-definition?latestVersion=true&sortBy=key&sortOrder=asc&maxResults={SEARCH_DEFINITION_LIMIT}",
            connection.rest_url
        );
        let result = async {
            let mut seen = HashSet::new();
            let mut definitions: Vec<ProcessDefinitionSummary> = parse_definitions(&self.get(&url).await?)?
                .into_iter()
                .filter(|definition| seen.insert((definition.key.clone(), definition.tenant_id.clone())))
                .collect();
            definitions.sort_by_cached_key(|definition| definition.display_name().to_lowercase());
            Ok(definitions)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &url))
    }
}

pub(super) fn parse_definitions(body: &str) -> ApiResult<Vec<ProcessDefinitionSummary>> {
    json_array(body)?.iter().map(parse_definition).collect()
}

pub(super) fn parse_definition(value: &Value) -> ApiResult<ProcessDefinitionSummary> {
    Ok(ProcessDefinitionSummary {
        id: string(value, "id").ok_or_else(|| ApiError::Invalid("Camunda не вернула ID process definition".to_string()))?,
        key: string(value, "key").ok_or_else(|| ApiError::Invalid("Camunda не вернула key process definition".to_string()))?,
        name: string(value, "name"),
        version: int(value, "version"),
        tenant_id: string(value, "tenantId"),
        suspended: boolean(value, "suspended"),
        deployment_id: string(value, "deploymentId"),
        deployment_time: None,
        instances: 0,
        incidents: 0,
        version_tag: string(value, "versionTag"),
        category: string(value, "category"),
        description: string(value, "description"),
        resource: string(value, "resource"),
        diagram_resource: string(value, "diagram"),
        history_time_to_live: nullable_int(value, "historyTimeToLive"),
        startable_in_tasklist: nullable_boolean(value, "startableInTasklist").unwrap_or(true),
    })
}

const SEARCH_DEFINITION_LIMIT: usize = 500;
