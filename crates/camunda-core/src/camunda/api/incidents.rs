use chrono::Utc;

use super::json::{json_array, string};
use super::{ApiResult, CamundaApi};
use crate::camunda::dates::{self, CamundaDateRange};
use crate::camunda::http::{self, ApiError, encode};
use crate::models::*;

impl CamundaApi {
    pub async fn load_incidents(
        &self,
        connection: &CamundaConnection,
        process_definition_id: Option<&str>,
        date_filter: &DashboardDateFilter,
        process_definition_key: Option<&str>,
    ) -> Result<Vec<ProcessIncident>, String> {
        let diagnostic_url = format!("{}/incident", connection.rest_url);
        let result = async {
            let mut incidents = if date_filter.is_live() {
                self.load_runtime_incidents(connection, process_definition_id, process_definition_key).await?
            } else {
                let range = dates::camunda_date_range(date_filter).map_err(ApiError::Invalid)?;
                self.load_historic_incidents(connection, &range, process_definition_id, process_definition_key).await?
            };
            incidents.sort_by_cached_key(|incident| {
                std::cmp::Reverse(dates::parse_camunda_date_time(incident.timestamp.as_deref()).map(|value| value.with_timezone(&Utc)))
            });
            Ok(incidents)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &diagnostic_url))
    }

    pub(super) async fn load_historic_incidents(
        &self,
        connection: &CamundaConnection,
        range: &CamundaDateRange,
        process_definition_id: Option<&str>,
        process_definition_key: Option<&str>,
    ) -> ApiResult<Vec<ProcessIncident>> {
        let definition_filter = process_definition_id.map(|id| format!("&processDefinitionId={}", encode(id))).unwrap_or_default();
        let key_filter = process_definition_key.map(|key| format!("&processDefinitionKeyIn={}", encode(key))).unwrap_or_default();
        let base_url = format!(
            "{}/history/incident?createTimeAfter={}&createTimeBefore={}{definition_filter}{key_filter}&sortBy=createTime&sortOrder=asc",
            connection.rest_url,
            encode(&range.after),
            encode(&range.before)
        );
        self.load_paged(&base_url, parse_plain_incidents).await
    }

    pub(super) async fn load_runtime_incidents(
        &self,
        connection: &CamundaConnection,
        process_definition_id: Option<&str>,
        process_definition_key: Option<&str>,
    ) -> ApiResult<Vec<ProcessIncident>> {
        let filters: Vec<String> = [
            process_definition_id.map(|id| format!("processDefinitionId={}", encode(id))),
            process_definition_key.map(|key| format!("processDefinitionKeyIn={}", encode(key))),
        ]
        .into_iter()
        .flatten()
        .collect();
        let filter = if filters.is_empty() { String::new() } else { format!("?{}", filters.join("&")) };
        let base_url = format!("{}/incident{filter}", connection.rest_url);
        self.load_paged(&base_url, parse_plain_incidents).await
    }
}

#[derive(Debug, Clone)]
pub(super) struct ProcessIncidentWithInstance {
    pub(super) process_instance_id: String,
    pub(super) incident: ProcessIncident,
}

pub(super) fn parse_incidents(body: &str) -> ApiResult<Vec<ProcessIncidentWithInstance>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            let id = string(value, "id")?;
            Some(ProcessIncidentWithInstance {
                process_instance_id: string(value, "processInstanceId").unwrap_or_default(),
                incident: ProcessIncident {
                    id,
                    incident_type: string(value, "incidentType").unwrap_or_else(|| "unknown".to_string()),
                    activity_id: string(value, "activityId"),
                    failed_activity_id: string(value, "failedActivityId"),
                    message: string(value, "incidentMessage"),
                    timestamp: string(value, "incidentTimestamp").or_else(|| string(value, "createTime")),
                    configuration: string(value, "configuration"),
                    process_definition_id: string(value, "processDefinitionId"),
                    process_instance_id: string(value, "processInstanceId"),
                    execution_id: string(value, "executionId"),
                    tenant_id: string(value, "tenantId"),
                    annotation: string(value, "annotation"),
                },
            })
        })
        .collect())
}

pub(super) fn parse_plain_incidents(body: &str) -> ApiResult<Vec<ProcessIncident>> {
    Ok(parse_incidents(body)?.into_iter().map(|incident| incident.incident).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn incidents_can_be_filtered_by_day_and_all_versions_of_process() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[
                {"id":"alpha:3:new","key":"alpha","name":"Alpha","version":3,"versionTag":"release-3","deploymentId":"dep-3","suspended":false},
                {"id":"alpha:2:old","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/incident",
            200,
            r#"[{"id":"incident-1","incidentType":"failedJob","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:old","createTime":"2026-07-15T09:00:00.000+0800"}]"#,
        )
        .await;
        let connection = connection(&server);

        let versions = api().load_process_definition_versions(&connection, "alpha").await.expect("версии");
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version_tag.as_deref(), Some("release-3"));
        let definition_query = query(&requests_to(&server, "/engine-rest/process-definition").await[0]);
        assert!(definition_query.contains("key=alpha"), "{definition_query}");
        assert!(definition_query.contains("sortBy=version"), "{definition_query}");

        let incidents =
            api().load_incidents(&connection, None, &custom("2026-07-15", "2026-07-15"), Some("alpha")).await.expect("инциденты");
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].timestamp.as_deref(), Some("2026-07-15T09:00:00.000+0800"));
        let incident_query = query(&requests_to(&server, "/engine-rest/history/incident").await[0]);
        assert!(incident_query.contains("processDefinitionKeyIn=alpha"), "{incident_query}");
        assert!(incident_query.contains("createTimeAfter="), "{incident_query}");
        assert!(incident_query.contains("createTimeBefore="), "{incident_query}");
    }

    #[tokio::test]
    async fn live_incidents_use_runtime_endpoint_sorted_newest_first() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/incident",
            200,
            r#"[
                {"id":"no-time","incidentType":"failedJob"},
                {"id":"old","incidentType":"failedJob","incidentTimestamp":"2026-07-15T09:00:00.000+0800"},
                {"id":"new","incidentType":"failedExternalTask","incidentTimestamp":"2026-07-16T09:00:00.000+0800"}
            ]"#,
        )
        .await;

        let incidents = api()
            .load_incidents(&connection(&server), Some("alpha:2:abc"), &DashboardDateFilter::default(), None)
            .await
            .expect("инциденты");

        assert_eq!(incidents.iter().map(|incident| incident.id.as_str()).collect::<Vec<_>>(), vec!["new", "old", "no-time"]);
        let incident_query = query(&requests_to(&server, "/engine-rest/incident").await[0]);
        assert!(incident_query.starts_with("processDefinitionId=alpha%3A2%3Aabc&firstResult=0&maxResults=1000"), "{incident_query}");
    }
}
