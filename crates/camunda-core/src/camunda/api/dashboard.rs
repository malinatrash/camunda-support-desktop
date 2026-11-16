use chrono::Utc;
use futures_util::FutureExt;

use super::activity_history::HistoricActivityInstance;
use super::definitions::parse_definitions;
use super::grouping::count_in_order;
use super::json::parse_count;
use super::statistics::{parse_deployment_times, parse_statistics};
use super::{ApiResult, CamundaApi};
use crate::camunda::dates;
use crate::camunda::http::{self, ApiError, encode};
use crate::models::*;

impl CamundaApi {
    pub async fn load_dashboard(
        &self,
        connection: &CamundaConnection,
        date_filter: &DashboardDateFilter,
    ) -> Result<ProcessDashboard, String> {
        let definitions_url = format!("{}/process-definition?sortBy=deployTime&sortOrder=desc", connection.rest_url);
        self.dashboard(connection, date_filter, &definitions_url).await.map_err(|error| http::user_message(&error, &definitions_url))
    }

    // Каждая версия приходит отдельно со своими счётчиками и датой деплоя, в процесс их сворачивает интерфейс.
    async fn dashboard(
        &self,
        connection: &CamundaConnection,
        date_filter: &DashboardDateFilter,
        definitions_url: &str,
    ) -> ApiResult<ProcessDashboard> {
        let rest_url = &connection.rest_url;
        let range = dates::dashboard_range(date_filter).map_err(ApiError::Invalid)?;
        let live = date_filter.is_live();
        let statistics_url = format!("{rest_url}/process-definition/statistics?incidents=true");
        let deployments_url = format!("{rest_url}/deployment?sortBy=deploymentTime&sortOrder=desc");
        let completed_count_url = format!(
            "{rest_url}/history/process-instance/count?completed=true&finishedAfter={}&finishedBefore={}",
            encode(&range.after),
            encode(&range.before)
        );

        let definitions = async { parse_definitions(&self.get(definitions_url).await?) }.shared();
        let statistics = async {
            if live {
                parse_statistics(&self.get(&statistics_url).await?)
            } else {
                let definitions = definitions.clone().await?;
                self.load_historic_statistics(connection, &definitions, date_filter).await
            }
        };
        let incidents = async {
            if live {
                self.load_runtime_incidents(connection, None, None).await.unwrap_or_default()
            } else {
                self.load_historic_incidents(connection, &range, None, None).await.unwrap_or_default()
            }
        };
        let unfinished_activities = async { self.load_unfinished_activities(connection).await.unwrap_or_default() };
        let completed_count = async { self.get(&completed_count_url).await.ok().and_then(|body| parse_count(&body).ok()) };
        let deployment_times =
            async { self.get(&deployments_url).await.ok().and_then(|body| parse_deployment_times(&body).ok()).unwrap_or_default() };

        let (definitions, statistics, incidents, unfinished, completed_count, deployment_times) =
            tokio::join!(definitions.clone(), statistics, incidents, unfinished_activities, completed_count, deployment_times);
        let definitions = definitions?;
        let statistics = statistics?;

        let mut attention_incidents: Vec<ProcessIncident> = incidents.iter().take(DASHBOARD_INCIDENT_LIMIT).cloned().collect();
        attention_incidents.sort_by_cached_key(|incident| {
            std::cmp::Reverse(dates::parse_camunda_date_time(incident.timestamp.as_deref()).map(|value| value.with_timezone(&Utc)))
        });
        let (range_start, range_end) = (range.start.with_timezone(&Utc), range.end.with_timezone(&Utc));
        let waiting_candidates: Vec<&HistoricActivityInstance> =
            unfinished.iter().filter(|activity| live || dates::is_inside(activity.start_time.as_deref(), range_start, range_end)).collect();
        let (incident_activities, longest_waiting_activities) = tokio::join!(
            self.load_incident_activities(connection, &incidents),
            self.load_longest_waiting_activities(connection, &waiting_candidates)
        );

        let mut incident_types: Vec<DashboardIncidentType> =
            count_in_order(incidents.iter().map(|incident| incident.incident_type.clone()))
                .into_iter()
                .map(|(incident_type, count)| DashboardIncidentType { incident_type, count })
                .collect();
        incident_types.sort_by_key(|item| std::cmp::Reverse(item.count));

        Ok(ProcessDashboard {
            definitions: definitions
                .into_iter()
                .map(|definition| {
                    let counts = statistics.get(&definition.id).copied().unwrap_or_default();
                    let deployment_time = definition.deployment_id.as_ref().and_then(|id| deployment_times.get(id)).cloned();
                    ProcessDefinitionSummary { instances: counts.instances, incidents: counts.incidents, deployment_time, ..definition }
                })
                .collect(),
            incident_types,
            incident_activities,
            longest_waiting_activities,
            completed_instances: completed_count.unwrap_or_default(),
            incidents: attention_incidents,
        })
    }
}

const DASHBOARD_INCIDENT_LIMIT: usize = 500;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn dashboard_lists_every_process_version_with_own_statistics() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[
                {"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null},
                {"id":"beta:4:def","key":"beta","name":null,"version":4,"deploymentId":"dep-4","suspended":true,"tenantId":"b2c"},
                {"id":"alpha:1:old","key":"alpha","name":"Alpha","version":1,"deploymentId":"dep-1","suspended":false,"tenantId":null}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/statistics",
            200,
            r#"[
                {"id":"alpha:2:abc","instances":6,"incidents":[{"incidentType":"failedJob","incidentCount":2}],"definition":{"key":"alpha","tenantId":null}},
                {"id":"alpha:1:old","instances":4,"incidents":[{"incidentType":"failedJob","incidentCount":1}],"definition":{"key":"alpha","tenantId":null}},
                {"id":"beta:4:def","instances":24,"incidents":[{"incidentType":"failedJob","incidentCount":3},{"incidentType":"externalTask","incidentCount":1}],"definition":{"key":"beta","tenantId":"b2c"}}
            ]"#,
        )
        .await;

        mount(
            &server,
            "GET",
            "/engine-rest/deployment",
            200,
            r#"[{"id":"dep-2","deploymentTime":"2026-10-30T12:00:00.000+0500"},{"id":"dep-1","deploymentTime":"2026-09-01T09:00:00.000+0500"}]"#,
        )
        .await;

        let dashboard = api().load_dashboard(&connection(&server), &DashboardDateFilter::default()).await.expect("дашборд");

        assert_eq!(dashboard.definitions.len(), 3);
        assert_eq!(dashboard.definitions[0].deployment_time.as_deref(), Some("2026-10-30T12:00:00.000+0500"));
        assert_eq!(dashboard.definitions[1].deployment_time, None);
        assert_eq!((dashboard.definitions[0].instances, dashboard.definitions[0].incidents), (6, 2));
        assert_eq!((dashboard.definitions[2].instances, dashboard.definitions[2].incidents), (4, 1), "старая версия своей строкой");
        assert_eq!(sum_instances(&dashboard), 34);
        assert_eq!(sum_incidents(&dashboard), 7);
        assert_eq!(dashboard.definitions[1].display_name(), "beta");
        assert!(dashboard.definitions[1].suspended);
        assert_eq!(dashboard.completed_instances, 0);
        let definition_query = query(&requests_to(&server, "/engine-rest/process-definition").await[0]);
        assert!(!definition_query.contains("latestVersion"), "{definition_query}");
        assert!(definition_query.contains("sortBy=deployTime"), "{definition_query}");
        assert!(definition_query.contains("sortOrder=desc"), "{definition_query}");
    }

    #[tokio::test]
    async fn forbidden_dashboard_returns_specific_failure() {
        let server = MockServer::start().await;
        mount(&server, "GET", "/engine-rest/process-definition", 403, "Forbidden").await;

        let message = api().load_dashboard(&connection(&server), &DashboardDateFilter::default()).await.expect_err("ошибка");

        assert!(message.contains("Access denied (HTTP 403)"), "{message}");
        assert!(message.contains("\nRequest: http://"), "{message}");
        assert!(message.contains("/engine-rest/process-definition?sortBy=deployTime"), "{message}");
    }
}
