use super::CamundaApi;
use super::grouping::count_in_order;
use crate::models::*;

impl CamundaApi {
    pub(super) async fn load_incident_activities(
        &self,
        connection: &CamundaConnection,
        incidents: &[ProcessIncident],
    ) -> Vec<DashboardIncidentActivity> {
        let keys = incidents.iter().filter_map(|incident| {
            let activity_id = incident.failed_activity_id.as_ref().or(incident.activity_id.as_ref())?;
            Some((activity_id.clone(), incident.process_definition_id.clone()))
        });
        let mut aggregated = count_in_order(keys);
        aggregated.sort_by_key(|item| std::cmp::Reverse(item.1));
        aggregated.truncate(DASHBOARD_ACTIVITY_LIMIT);
        let definition_ids = aggregated.iter().filter_map(|((_, definition_id), _)| definition_id.clone()).collect();
        let diagrams = self.load_diagrams(connection, definition_ids).await;
        aggregated
            .into_iter()
            .map(|((activity_id, process_definition_id), count)| {
                let node = process_definition_id
                    .as_ref()
                    .and_then(|definition_id| diagrams.get(definition_id))
                    .and_then(|loaded| loaded.diagram.node(&activity_id));
                DashboardIncidentActivity {
                    activity_id,
                    process_definition_id,
                    count,
                    activity_name: node.and_then(|node| node.name.clone()),
                    topic: node.and_then(|node| node.topic.clone()),
                }
            })
            .collect()
    }
}

const DASHBOARD_ACTIVITY_LIMIT: usize = 12;

#[cfg(test)]
mod tests {
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn dashboard_groups_incidents_and_waiting_activities_for_period() {
        let server = MockServer::start().await;
        let bpmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?><definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"><process id="alpha"><serviceTask id="sendDocuments" name="Отправить документы" /></process><bpmndi:BPMNDiagram><bpmndi:BPMNPlane bpmnElement="alpha"><bpmndi:BPMNShape bpmnElement="sendDocuments"><dc:Bounds x="100" y="100" width="100" height="80" /></bpmndi:BPMNShape></bpmndi:BPMNPlane></bpmndi:BPMNDiagram></definitions>"#;
        mount(&server, "GET", "/engine-rest/process-definition/alpha%3A2%3Aabc/xml", 200, &json!({ "bpmn20Xml": bpmn_xml }).to_string())
            .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}]"#,
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/engine-rest/history/process-instance/count"))
            .and(has_completed_flag)
            .respond_with(json_response(200, r#"{"count":2}"#))
            .mount(&server)
            .await;
        mount(&server, "GET", "/engine-rest/history/process-instance/count", 200, r#"{"count":3}"#).await;
        mount(&server, "GET", "/engine-rest/history/incident/count", 200, r#"{"count":2}"#).await;
        Mock::given(method("GET"))
            .and(path("/engine-rest/history/process-instance"))
            .and(has_completed_flag)
            .respond_with(json_response(
                200,
                r#"[
                    {"id":"done-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T10:00:00.000+0000","endTime":"2026-07-15T10:30:00.000+0000","state":"COMPLETED"},
                    {"id":"done-2","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-16T10:00:00.000+0000","endTime":"2026-07-16T12:00:00.000+0000","state":"COMPLETED"}
                ]"#,
            ))
            .mount(&server)
            .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[
                {"id":"started-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T10:00:00.000+0000"},
                {"id":"started-2","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T13:00:00.000+0000"},
                {"id":"started-3","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-16T10:00:00.000+0000"}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/incident",
            200,
            r#"[
                {"id":"incident-1","incidentType":"failedJob","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:abc","processInstanceId":"started-1","createTime":"2026-07-15T11:00:00.000+0000"},
                {"id":"incident-2","incidentType":"failedExternalTask","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:abc","processInstanceId":"started-3","createTime":"2026-07-16T11:00:00.000+0000"}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/activity-instance",
            200,
            r#"[
                {"id":"activity-ancient","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"ancient-300-days","startTime":"2025-09-19T10:00:00.000+0000"},
                {"id":"activity-old","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"started-1","startTime":"2026-07-15T10:00:00.000+0000"},
                {"id":"activity-new","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"started-3","startTime":"2026-07-16T10:00:00.000+0000"}
            ]"#,
        )
        .await;
        let repository: Arc<dyn ProcessVariableCatalogRepository> = Arc::new(InMemoryProcessVariableCatalogRepository::default());

        let api = api_at("2026-07-16T12:00:00Z", repository);

        let dashboard = api.load_dashboard(&connection(&server), &custom("2026-07-15", "2026-07-16")).await.expect("дашборд");

        assert_eq!(sum_instances(&dashboard), 3);
        assert_eq!(dashboard.completed_instances, 2);
        assert_eq!(sum_incidents(&dashboard), 2);
        let incident_types: HashSet<&str> = dashboard.incident_types.iter().map(|value| value.incident_type.as_str()).collect();
        assert_eq!(incident_types, HashSet::from(["failedJob", "failedExternalTask"]));
        assert_eq!(dashboard.incident_activities.len(), 1);
        assert_eq!(dashboard.incident_activities[0].activity_id, "sendDocuments");
        assert_eq!(dashboard.incident_activities[0].activity_name.as_deref(), Some("Отправить документы"));
        assert_eq!(dashboard.incident_activities[0].count, 2);
        assert!(query(&requests_to(&server, "/engine-rest/history/activity-instance").await[0]).contains("unfinished=true"));
        assert_eq!(dashboard.longest_waiting_activities.len(), 1);
        let waiting = &dashboard.longest_waiting_activities[0];
        assert_eq!(waiting.activity_name.as_deref(), Some("Отправить документы"));
        assert_eq!(waiting.instance_count, 2);
        assert_eq!(waiting.oldest_process_instance_id, "started-1");
        assert_eq!(waiting.maximum_waiting_millis, 93_600_000);
        assert_eq!(waiting.average_waiting_millis, 50_400_000);
        assert_eq!(waiting.oldest_started_at.as_deref(), Some("2026-07-15T10:00:00.000+0000"));
        let xml_route = "/engine-rest/process-definition/alpha%3A2%3Aabc/xml";
        let xml_requests = requests_to(&server, xml_route).await.len();
        assert!((1..=2).contains(&xml_requests), "{xml_requests}");
        api.load_dashboard(&connection(&server), &custom("2026-07-15", "2026-07-16")).await.expect("повторный дашборд");
        assert_eq!(requests_to(&server, xml_route).await.len(), xml_requests, "BPMN берётся из кэша при повторной загрузке");
    }
}
