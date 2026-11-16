use serde_json::json;

use super::CamundaApi;
use crate::models::*;

impl CamundaApi {
    // Массово выставляет retries заданиям и внешним задачам из списка инцидентов одним запросом на тип.
    pub async fn retry_incidents(
        &self,
        connection: &CamundaConnection,
        incidents: &[ProcessIncident],
        retries: i32,
    ) -> Result<BulkRetryResult, String> {
        let mut job_ids: Vec<&str> = Vec::new();
        let mut external_task_ids: Vec<&str> = Vec::new();
        let mut skipped = 0;
        for incident in incidents {
            match (incident.incident_type.as_str(), incident.configuration.as_deref().filter(|id| !id.trim().is_empty())) {
                ("failedJob", Some(id)) => job_ids.push(id),
                ("failedExternalTask", Some(id)) => external_task_ids.push(id),
                _ => skipped += 1,
            }
        }
        job_ids.sort_unstable();
        job_ids.dedup();
        external_task_ids.sort_unstable();
        external_task_ids.dedup();
        if job_ids.is_empty() && external_task_ids.is_empty() {
            return Err(rust_i18n::t!("core.incidents.nothing_to_retry").to_string());
        }
        if !job_ids.is_empty() {
            let url = format!("{}/job/retries", connection.rest_url);
            self.mutate(&url, reqwest::Method::PUT, Some(json!({ "jobIds": job_ids, "retries": retries }))).await?;
        }
        if !external_task_ids.is_empty() {
            let url = format!("{}/external-task/retries", connection.rest_url);
            self.mutate(&url, reqwest::Method::PUT, Some(json!({ "externalTaskIds": external_task_ids, "retries": retries }))).await?;
        }
        Ok(BulkRetryResult { jobs: job_ids.len() as i32, external_tasks: external_task_ids.len() as i32, skipped })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn bulk_retry_groups_incidents_into_two_requests() {
        let server = MockServer::start().await;
        mount(&server, "PUT", "/engine-rest/job/retries", 204, "").await;
        mount(&server, "PUT", "/engine-rest/external-task/retries", 204, "").await;
        let incident = |kind: &str, configuration: Option<&str>| ProcessIncident {
            id: format!("{kind}-{}", configuration.unwrap_or("none")),
            incident_type: kind.to_string(),
            activity_id: None,
            failed_activity_id: None,
            message: None,
            timestamp: None,
            configuration: configuration.map(str::to_string),
            process_definition_id: None,
            process_instance_id: None,
            execution_id: None,
            tenant_id: None,
            annotation: None,
        };
        let incidents = vec![
            incident("failedJob", Some("job-1")),
            incident("failedJob", Some("job-1")),
            incident("failedExternalTask", Some("ext-7")),
            incident("failedJob", None),
            incident("unknown", Some("x")),
        ];

        let result = api().retry_incidents(&connection(&server), &incidents, 3).await.expect("повтор");

        assert_eq!(result, BulkRetryResult { jobs: 1, external_tasks: 1, skipped: 2 });
        let jobs = requests_to(&server, "/engine-rest/job/retries").await;
        assert_eq!(body_json(&jobs[0]), json!({ "jobIds": ["job-1"], "retries": 3 }));
        let tasks = requests_to(&server, "/engine-rest/external-task/retries").await;
        assert_eq!(body_json(&tasks[0]), json!({ "externalTaskIds": ["ext-7"], "retries": 3 }));
        assert!(api().retry_incidents(&connection(&server), &[incident("unknown", None)], 3).await.is_err());
    }
}
