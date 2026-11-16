use serde_json::json;

use super::CamundaApi;
use crate::camunda::http::encode;
use crate::models::*;

impl CamundaApi {
    pub async fn set_job_retries(&self, connection: &CamundaConnection, job_id: &str, retries: i32) -> Result<(), String> {
        let url = format!("{}/job/{}/retries", connection.rest_url, encode(job_id));
        self.mutate(&url, reqwest::Method::PUT, Some(json!({ "retries": retries }))).await
    }

    pub async fn set_external_task_retries(
        &self,
        connection: &CamundaConnection,
        external_task_id: &str,
        retries: i32,
    ) -> Result<(), String> {
        let url = format!("{}/external-task/{}/retries", connection.rest_url, encode(external_task_id));
        self.mutate(&url, reqwest::Method::PUT, Some(json!({ "retries": retries }))).await
    }

    pub async fn unlock_external_task(&self, connection: &CamundaConnection, external_task_id: &str) -> Result<(), String> {
        let url = format!("{}/external-task/{}/unlock", connection.rest_url, encode(external_task_id));
        self.mutate(&url, reqwest::Method::POST, None).await
    }

    pub async fn set_process_instance_suspended(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
        suspended: bool,
    ) -> Result<(), String> {
        let url = format!("{}/process-instance/{}/suspended", connection.rest_url, encode(process_instance_id));
        self.mutate(&url, reqwest::Method::PUT, Some(json!({ "suspended": suspended }))).await
    }
}

#[cfg(test)]
mod tests {
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn recovery_operations_use_expected_endpoints_and_bodies() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;
        let connection = connection(&server);
        let api = api();

        api.set_job_retries(&connection, "job-1", 3).await.expect("retries job");
        api.set_external_task_retries(&connection, "task-1", 3).await.expect("retries external task");
        api.unlock_external_task(&connection, "task-1").await.expect("unlock");
        api.set_process_instance_suspended(&connection, "instance-1", true).await.expect("suspend");

        let requests = server.received_requests().await.unwrap_or_default();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].method.as_str(), "PUT");
        assert_eq!(requests[0].url.path(), "/engine-rest/job/job-1/retries");
        assert_eq!(body_json(&requests[0])["retries"], 3);
        assert_eq!(requests[1].method.as_str(), "PUT");
        assert_eq!(requests[1].url.path(), "/engine-rest/external-task/task-1/retries");
        assert_eq!(body_json(&requests[1])["retries"], 3);
        assert_eq!(requests[2].method.as_str(), "POST");
        assert_eq!(requests[2].url.path(), "/engine-rest/external-task/task-1/unlock");
        assert!(requests[2].body.is_empty());
        assert_eq!(requests[3].method.as_str(), "PUT");
        assert_eq!(requests[3].url.path(), "/engine-rest/process-instance/instance-1/suspended");
        assert_eq!(body_json(&requests[3])["suspended"], true);
    }
}
