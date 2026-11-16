use std::collections::HashMap;

use futures_util::future::try_join_all;
use tokio::sync::Semaphore;

use super::json::{as_array, int, json_array, parse_count, string};
use super::{ApiResult, CamundaApi, MAX_PARALLEL_HISTORY_REQUESTS};
use crate::camunda::dates;
use crate::camunda::http::{ApiError, encode};
use crate::models::*;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct DefinitionStatistics {
    pub(super) instances: i32,
    pub(super) incidents: i32,
}

impl CamundaApi {
    pub(super) async fn load_historic_statistics(
        &self,
        connection: &CamundaConnection,
        definitions: &[ProcessDefinitionSummary],
        date_filter: &DashboardDateFilter,
    ) -> ApiResult<HashMap<String, DefinitionStatistics>> {
        let range = dates::camunda_date_range(date_filter).map_err(ApiError::Invalid)?;
        let after = encode(&range.after);
        let before = encode(&range.before);
        let semaphore = Semaphore::new(MAX_PARALLEL_HISTORY_REQUESTS);
        let loads = definitions.iter().map(|definition| {
            let (semaphore, after, before) = (&semaphore, &after, &before);
            async move {
                let _permit =
                    semaphore.acquire().await.map_err(|_| ApiError::Other(rust_i18n::t!("core.http.semaphore_closed").to_string()))?;
                let filter = format!("processDefinitionId={}", encode(&definition.id));
                let instances_url =
                    format!("{}/history/process-instance/count?{filter}&startedAfter={after}&startedBefore={before}", connection.rest_url);
                let incidents_url =
                    format!("{}/history/incident/count?{filter}&createTimeAfter={after}&createTimeBefore={before}", connection.rest_url);
                let statistics = DefinitionStatistics {
                    instances: parse_count(&self.get(&instances_url).await?)?,
                    incidents: parse_count(&self.get(&incidents_url).await?)?,
                };
                Ok::<_, ApiError>((definition.id.clone(), statistics))
            }
        });
        Ok(try_join_all(loads).await?.into_iter().collect())
    }
}

pub(super) fn parse_statistics(body: &str) -> ApiResult<HashMap<String, DefinitionStatistics>> {
    let mut result: HashMap<String, DefinitionStatistics> = HashMap::new();
    for value in json_array(body)? {
        let Some(id) = string(&value, "id") else { continue };
        let entry = result.entry(id).or_default();
        entry.instances += int(&value, "instances");
        entry.incidents += as_array(value.get("incidents")).iter().map(|incident| int(incident, "incidentCount")).sum::<i32>();
    }
    Ok(result)
}

pub(super) fn parse_deployment_times(body: &str) -> ApiResult<HashMap<String, String>> {
    Ok(json_array(body)?.iter().filter_map(|value| Some((string(value, "id")?, string(value, "deploymentTime")?))).collect())
}

#[cfg(test)]
mod tests {
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn custom_period_uses_historic_count_endpoints() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}]"#,
        )
        .await;
        mount(&server, "GET", "/engine-rest/history/process-instance/count", 200, r#"{"count":17}"#).await;
        mount(&server, "GET", "/engine-rest/history/incident/count", 200, r#"{"count":3}"#).await;

        let dashboard = api().load_dashboard(&connection(&server), &custom("2026-07-15", "2026-07-16")).await.expect("дашборд");

        assert_eq!(sum_instances(&dashboard), 17);
        assert_eq!(sum_incidents(&dashboard), 3);
        let process_queries: Vec<String> =
            requests_to(&server, "/engine-rest/history/process-instance/count").await.iter().map(query).collect();
        let definition_query = process_queries.iter().find(|query| query.contains("processDefinitionId=")).expect("запрос по версии");
        assert!(definition_query.contains("processDefinitionId=alpha%3A2%3Aabc"), "{definition_query}");
        assert!(definition_query.contains("startedAfter="), "{definition_query}");
        assert!(definition_query.contains("startedBefore="), "{definition_query}");
        let incident_query = query(&requests_to(&server, "/engine-rest/history/incident/count").await[0]);
        assert!(incident_query.contains("processDefinitionId=alpha%3A2%3Aabc"), "{incident_query}");
        assert!(incident_query.contains("createTimeAfter="), "{incident_query}");
        assert!(incident_query.contains("createTimeBefore="), "{incident_query}");
    }
}
