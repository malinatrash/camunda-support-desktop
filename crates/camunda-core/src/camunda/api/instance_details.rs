use super::activity_history::{parse_historic_activity_instances, to_activity_execution_summaries, to_activity_timeline};
use super::current_activities::{build_current_activities, parse_activity_tree};
use super::incidents::parse_incidents;
use super::instances::parse_instance;
use super::jobs::{parse_external_tasks, parse_jobs};
use super::json::json_object;
use super::metadata::instance_metadata;
use super::variables::parse_variables;
use super::{CamundaApi, DETAIL_LIST_LIMIT};
use crate::camunda::http::{self, ApiError, encode};
use crate::models::*;

impl CamundaApi {
    pub async fn load_process_instance_details(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
    ) -> Result<ProcessInstanceDetails, String> {
        let rest_url = &connection.rest_url;
        let path_id = encode(process_instance_id);
        let query_id = encode(process_instance_id);
        let instance_url = format!("{rest_url}/process-instance/{path_id}");
        let result = async {
            let instance_json = json_object(&self.get(&instance_url).await?)?;
            let instance = parse_instance(&instance_json)?;
            let variables = async {
                parse_variables(&self.get(&format!("{rest_url}/process-instance/{path_id}/variables?deserializeValues=false")).await?)
            };
            let activities =
                async { parse_activity_tree(&self.get(&format!("{rest_url}/process-instance/{path_id}/activity-instances")).await?) };
            let activity_history = async {
                let history = self
                    .load_paged(
                        &format!("{rest_url}/history/activity-instance?processInstanceId={query_id}&sortBy=startTime&sortOrder=asc"),
                        parse_historic_activity_instances,
                    )
                    .await?;
                Ok::<_, ApiError>((to_activity_execution_summaries(&history), to_activity_timeline(&history)))
            };
            let incidents = async {
                let incidents = parse_incidents(
                    &self.get(&format!("{rest_url}/incident?processInstanceId={query_id}&maxResults={DETAIL_LIST_LIMIT}")).await?,
                )?;
                Ok::<_, ApiError>(incidents.into_iter().map(|incident| incident.incident).collect::<Vec<_>>())
            };
            let jobs = async {
                parse_jobs(
                    &self
                        .get(&format!("{rest_url}/job?processInstanceId={query_id}&withException=true&maxResults={DETAIL_LIST_LIMIT}"))
                        .await?,
                )
            };
            let external_tasks = async {
                parse_external_tasks(
                    &self.get(&format!("{rest_url}/external-task?processInstanceId={query_id}&maxResults={DETAIL_LIST_LIMIT}")).await?,
                )
            };
            let bpmn = self.load_bpmn(connection, &instance.definition_id);
            let (variables, activities, (activity_history, activity_timeline), incidents, jobs, external_tasks, bpmn) =
                tokio::try_join!(variables, activities, activity_history, incidents, jobs, external_tasks, bpmn)?;
            let active_activities =
                build_current_activities(activities, &activity_history, &incidents, &jobs, &external_tasks, &bpmn.diagram);
            Ok(ProcessInstanceDetails {
                instance,
                variables,
                active_activities,
                activity_history,
                incidents,
                jobs,
                external_tasks,
                diagram: bpmn.diagram.clone(),
                bpmn_xml: bpmn.xml.clone(),
                metadata: instance_metadata(&instance_json),
                activity_timeline,
            })
        }
        .await;
        result.map_err(|error| http::user_message(&error, &instance_url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn instance_details_combine_runtime_state_variables_failures_and_bpmn() {
        let server = MockServer::start().await;
        let bpmn_xml = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
    xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
    xmlns:dc="http://www.omg.org/spec/DD/20100524/DC">
  <process id="loan">
    <serviceTask id="Task_1" name="Проверка" />
    <serviceTask id="Task_2" name="Ожидание перехода" />
    <userTask id="Task_4" name="Ожидание клиента" />
  </process>
  <bpmndi:BPMNDiagram id="d"><bpmndi:BPMNPlane id="p" bpmnElement="loan">
    <bpmndi:BPMNShape id="s" bpmnElement="Task_1"><dc:Bounds x="10" y="20" width="100" height="80" /></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="s2" bpmnElement="Task_2"><dc:Bounds x="150" y="20" width="100" height="80" /></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="s4" bpmnElement="Task_4"><dc:Bounds x="290" y="20" width="100" height="80" /></bpmndi:BPMNShape>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance/instance-1",
            200,
            r#"{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","businessKey":"APP-42","ended":false,"suspended":false,"tenantId":"b2c"}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance/instance-1/variables",
            200,
            r#"{"applicationId":{"type":"String","value":"APP-42","valueInfo":{}}}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance/instance-1/activity-instances",
            200,
            r#"{"id":"root","activityId":"loan","activityType":"processDefinition","childActivityInstances":[{"id":"activity-instance-1","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","executionIds":["exec-1"],"incidentIds":["incident-1"]}],"childTransitionInstances":[{"id":"transition-1","activityId":"Task_2","activityName":"Ожидание перехода","activityType":"serviceTask","executionId":"exec-2","incidentIds":[]}]}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/activity-instance",
            200,
            r#"[
                {"id":"historic-1","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:00:00.000+0000","endTime":"2026-07-16T09:00:01.000+0000","canceled":false},
                {"id":"historic-2","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:01:00.000+0000","endTime":"2026-07-16T09:01:01.000+0000","canceled":false},
                {"id":"historic-3","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:02:00.000+0000","endTime":null,"canceled":false},
                {"id":"historic-4","activityId":"Task_4","activityName":"Ожидание клиента","activityType":"userTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:03:00.000+0000","endTime":null,"canceled":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/incident",
            200,
            r#"[{"id":"incident-1","processInstanceId":"instance-1","incidentType":"failedJob","failedActivityId":"Task_1","incidentMessage":"boom"}]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/job",
            200,
            r#"[{"id":"job-1","retries":0,"failedActivityId":"Task_1","exceptionMessage":"boom","suspended":false}]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/external-task",
            200,
            r#"[{"id":"external-1","activityId":"Task_3","topicName":"loan-check","retries":0,"errorMessage":"worker failed","workerId":"worker-7","lockExpirationTime":"2026-07-16T10:00:00.000+0000","suspended":false}]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/loan%3A3%3Adef/xml",
            200,
            &json!({ "id": "loan:3:def", "bpmn20Xml": bpmn_xml }).to_string(),
        )
        .await;

        let details = api().load_process_instance_details(&connection(&server), "instance-1").await.expect("детали заявки");

        let activity =
            |activity_id: &str| details.active_activities.iter().find(|activity| activity.activity_id == activity_id).expect("активность");
        assert_eq!(details.instance.business_key.as_deref(), Some("APP-42"));
        assert_eq!(details.variables.len(), 1);
        assert_eq!(details.variables[0].value, "APP-42");
        assert_eq!(details.variables[0].value_type, "String");
        assert_eq!(details.variables[0].value_info, None);
        assert_eq!(details.active_activities.len(), 4);
        assert_eq!(activity("Task_1").id, "activity-instance-1");
        assert_eq!(
            activity("Task_1").evidence,
            vec![CurrentActivityEvidence::RuntimeActivity, CurrentActivityEvidence::Incident, CurrentActivityEvidence::Job]
        );
        assert!(activity("Task_1").cancellable);
        assert_eq!(activity("Task_1").incident_ids, vec!["incident-1".to_string()]);
        assert_eq!(activity("Task_2").evidence, vec![CurrentActivityEvidence::Transition]);
        assert_eq!(activity("Task_2").execution_ids, vec!["exec-2".to_string()]);
        assert_eq!(activity("Task_3").evidence, vec![CurrentActivityEvidence::ExternalTask]);
        assert_eq!(activity("Task_3").id, "diagnostic:Task_3");
        assert_eq!(activity("Task_3").activity_type, "unknown");
        assert!(!activity("Task_3").cancellable);
        assert_eq!(activity("Task_4").evidence, vec![CurrentActivityEvidence::UnfinishedHistory]);
        assert_eq!(activity("Task_4").activity_name.as_deref(), Some("Ожидание клиента"));
        assert_eq!(activity("Task_4").activity_type, "userTask");
        assert!(!activity("Task_4").cancellable);
        let task_1_history = details.activity_history.iter().find(|summary| summary.activity_id == "Task_1").expect("история Task_1");
        assert_eq!(task_1_history.completed_count, 2);
        assert_eq!(task_1_history.active_count, 1);
        assert_eq!(task_1_history.canceled_count, 0);
        assert_eq!(details.incidents.len(), 1);
        assert_eq!(details.incidents[0].message.as_deref(), Some("boom"));
        assert_eq!(details.jobs.len(), 1);
        assert_eq!(details.jobs[0].retries, 0);
        assert_eq!(details.external_tasks.len(), 1);
        assert_eq!(details.external_tasks[0].worker_id.as_deref(), Some("worker-7"));
        let node_ids: HashSet<&str> = details.diagram.nodes.iter().map(|node| node.id.as_str()).collect();
        assert_eq!(node_ids, HashSet::from(["Task_1", "Task_2", "Task_4"]));
        assert_eq!(details.bpmn_xml, bpmn_xml);
        let tenant = details.metadata.iter().find(|field| field.label == "Tenant ID").expect("метаданные");
        assert_eq!(tenant.value, "b2c");
        let case = details.metadata.iter().find(|field| field.label == "Case instance ID").expect("метаданные");
        assert_eq!(case.value, "—");
        let history_query = query(&requests_to(&server, "/engine-rest/history/activity-instance").await[0]);
        assert!(history_query.contains("processInstanceId=instance-1"), "{history_query}");
        assert!(
            query(&requests_to(&server, "/engine-rest/process-instance/instance-1/variables").await[0]).contains("deserializeValues=false")
        );
    }
}
