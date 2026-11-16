use std::collections::HashMap;

use super::definitions::parse_definition;
use super::grouping::count_in_order;
use super::incidents::parse_incidents;
use super::instances::parse_instances;
use super::json::json_object;
use super::metadata::definition_metadata;
use super::{CamundaApi, DETAIL_LIST_LIMIT};
use crate::camunda::dates;
use crate::camunda::http::{self, ApiError, encode};
use crate::models::*;

impl CamundaApi {
    // all_versions: заявки и инциденты всех версий процесса по ключу, иначе только выбранной версии.
    pub async fn load_process_definition_details(
        &self,
        connection: &CamundaConnection,
        process_definition_id: &str,
        date_filter: &DashboardDateFilter,
        all_versions: bool,
    ) -> Result<ProcessDefinitionDetails, String> {
        let rest_url = &connection.rest_url;
        let definition_url = format!("{rest_url}/process-definition/{}", encode(process_definition_id));
        let result = async {
            let range = if date_filter.is_live() {
                None
            } else {
                Some(dates::camunda_date_range(date_filter).map_err(ApiError::Invalid)?)
            };
            let definition_json = json_object(&self.get(&definition_url).await?)?;
            let definition = parse_definition(&definition_json)?;
            let (instance_filter, incident_filter, history_incident_filter) = if all_versions {
                let key_filter = definition_key_filter(&definition.key, definition.tenant_id.as_deref());
                let incident_filter = key_filter.replacen("processDefinitionKey=", "processDefinitionKeyIn=", 1);
                (key_filter.clone(), incident_filter, key_filter)
            } else {
                let id_filter = format!("processDefinitionId={}", encode(process_definition_id));
                (id_filter.clone(), id_filter.clone(), id_filter)
            };
            let instances_url = match &range {
                None => format!(
                    "{rest_url}/history/process-instance?{instance_filter}&unfinished=true&sortBy=startTime&sortOrder=desc&maxResults={DETAIL_LIST_LIMIT}"
                ),
                Some(range) => format!(
                    "{rest_url}/history/process-instance?{instance_filter}&startedAfter={}&startedBefore={}&sortBy=startTime&sortOrder=desc&maxResults={DETAIL_LIST_LIMIT}",
                    encode(&range.after),
                    encode(&range.before)
                ),
            };
            let incidents_url = match &range {
                None => format!("{rest_url}/incident?{incident_filter}&maxResults={DETAIL_LIST_LIMIT}"),
                Some(range) => format!(
                    "{rest_url}/history/incident?{history_incident_filter}&createTimeAfter={}&createTimeBefore={}&sortBy=createTime&sortOrder=desc&maxResults={DETAIL_LIST_LIMIT}",
                    encode(&range.after),
                    encode(&range.before)
                ),
            };
            let (instances_body, incidents_body) = tokio::try_join!(self.get(&instances_url), self.get(&incidents_url))?;
            let instances = parse_instances(&instances_body)?;
            let incidents = parse_incidents(&incidents_body)?;
            let incidents_by_instance: HashMap<&str, i32> =
                count_in_order(incidents.iter().map(|incident| incident.process_instance_id.as_str())).into_iter().collect();
            Ok(ProcessDefinitionDetails {
                definition: ProcessDefinitionSummary {
                    instances: instances.len() as i32,
                    incidents: incidents.len() as i32,
                    ..definition
                },
                instances: instances
                    .into_iter()
                    .map(|instance| {
                        let incident_count = incidents_by_instance.get(instance.id.as_str()).copied().unwrap_or(0);
                        RuntimeInstanceListItem { instance, incident_count }
                    })
                    .collect(),
                metadata: definition_metadata(&definition_json),
                date_filter: date_filter.clone(),
            })
        }
        .await;
        result.map_err(|error| http::user_message(&error, &definition_url))
    }
}

fn definition_key_filter(key: &str, tenant_id: Option<&str>) -> String {
    match tenant_id {
        Some(tenant_id) => format!("processDefinitionKey={}&tenantIdIn={}", encode(key), encode(tenant_id)),
        None => format!("processDefinitionKey={}", encode(key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn definition_period_loads_historic_instances_from_selected_dates() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/alpha%3A2%3Aabc",
            200,
            r#"{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[{"id":"historic-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","businessKey":"APP-YESTERDAY","startTime":"2026-07-15T10:00:00.000+0000","endTime":"2026-07-15T10:02:00.000+0000","state":"COMPLETED","tenantId":null}]"#,
        )
        .await;
        mount(&server, "GET", "/engine-rest/history/incident", 200, "[]").await;
        let filter = DashboardDateFilter { preset: DashboardDatePreset::Yesterday, ..DashboardDateFilter::default() };

        let details =
            api().load_process_definition_details(&connection(&server), "alpha:2:abc", &filter, false).await.expect("детали процесса");

        assert_eq!(details.instances.len(), 1);
        assert_eq!(details.instances[0].instance.business_key.as_deref(), Some("APP-YESTERDAY"));
        assert!(details.instances[0].instance.ended);
        assert_eq!(details.instances[0].incident_count, 0);
        assert_eq!(details.definition.instances, 1);
        assert_eq!(details.definition.incidents, 0);
        assert_eq!(details.metadata[0], ProcessMetadataField { label: "ID определения".to_string(), value: "alpha:2:abc".to_string() });
        let instance_query = query(&requests_to(&server, "/engine-rest/history/process-instance").await[0]);
        assert!(instance_query.contains("startedAfter="), "{instance_query}");
        assert!(instance_query.contains("startedBefore="), "{instance_query}");
        let incident_query = query(&requests_to(&server, "/engine-rest/history/incident").await[0]);
        assert!(incident_query.contains("sortBy=createTime"), "{incident_query}");
        assert_eq!(details.date_filter.preset, DashboardDatePreset::Yesterday);
    }

    #[tokio::test]
    async fn live_definition_details_count_runtime_incidents_per_instance() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/alpha%3A2%3Aabc",
            200,
            r#"{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[
                {"id":"i-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-11-06T10:00:00.000+0000","state":"ACTIVE"},
                {"id":"i-2","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-11-05T10:00:00.000+0000","state":"SUSPENDED"}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/incident",
            200,
            r#"[
                {"id":"inc-1","processInstanceId":"i-1","incidentType":"failedJob"},
                {"id":"inc-2","processInstanceId":"i-1","incidentType":"failedJob"}
            ]"#,
        )
        .await;

        let details = api()
            .load_process_definition_details(&connection(&server), "alpha:2:abc", &DashboardDateFilter::default(), false)
            .await
            .expect("детали процесса");

        assert_eq!(details.definition.instances, 2);
        assert_eq!(details.definition.incidents, 2);
        assert_eq!(details.instances.iter().map(|item| item.incident_count).collect::<Vec<_>>(), vec![2, 0]);
        assert!(details.instances[1].instance.suspended);
        assert_eq!(details.instances[0].instance.start_time.as_deref(), Some("2026-11-06T10:00:00.000+0000"));
        let instance_query = query(&requests_to(&server, "/engine-rest/history/process-instance").await[0]);
        assert!(instance_query.contains("processDefinitionId=alpha%3A2%3Aabc"), "{instance_query}");
        assert!(instance_query.contains("unfinished=true&sortBy=startTime&sortOrder=desc"), "{instance_query}");
        assert!(instance_query.contains("maxResults=500"), "{instance_query}");
    }

    #[tokio::test]
    async fn definition_details_can_cover_all_versions_by_key() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/alpha%3A3%3Anew",
            200,
            r#"{"id":"alpha:3:new","key":"alpha","name":"Alpha","version":3,"suspended":false,"tenantId":"b2c"}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[
                {"id":"i-new","processDefinitionId":"alpha:3:new","processDefinitionKey":"alpha"},
                {"id":"i-old","processDefinitionId":"alpha:1:old","processDefinitionKey":"alpha"}
            ]"#,
        )
        .await;
        mount(&server, "GET", "/engine-rest/incident", 200, r#"[{"id":"inc-1","processInstanceId":"i-old","incidentType":"failedJob"}]"#)
            .await;

        let details = api()
            .load_process_definition_details(&connection(&server), "alpha:3:new", &DashboardDateFilter::default(), true)
            .await
            .expect("детали процесса");

        assert_eq!(details.instances.len(), 2);
        assert_eq!(details.instances[1].incident_count, 1);
        let instance_query = query(&requests_to(&server, "/engine-rest/history/process-instance").await[0]);
        assert!(instance_query.contains("processDefinitionKey=alpha&tenantIdIn=b2c"), "{instance_query}");
        let incident_query = query(&requests_to(&server, "/engine-rest/incident").await[0]);
        assert!(incident_query.contains("processDefinitionKeyIn=alpha&tenantIdIn=b2c"), "{incident_query}");
    }
}
