use serde_json::json;

use super::CamundaApi;
use crate::camunda::http::encode;
use crate::models::*;

impl CamundaApi {
    pub async fn teleport_process_instance(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
        request: &TeleportRequest,
    ) -> Result<(), String> {
        let url = format!("{}/process-instance/{}/modification", connection.rest_url, encode(process_instance_id));
        let body = json!({
            "skipCustomListeners": request.skip_custom_listeners,
            "skipIoMappings": request.skip_io_mappings,
            "annotation": request.annotation,
            "instructions": [
                { "type": "cancel", "activityInstanceId": request.source_activity_instance_id },
                { "type": "startBeforeActivity", "activityId": request.target_activity_id },
            ],
        });
        self.mutate(&url, reqwest::Method::POST, Some(body)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn teleport_uses_activity_instance_id_and_start_target() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;

        api()
            .teleport_process_instance(
                &connection(&server),
                "instance-1",
                &TeleportRequest {
                    source_activity_instance_id: "activity-instance-7".to_string(),
                    target_activity_id: "Activity_target".to_string(),
                    annotation: "INC-42".to_string(),
                    skip_custom_listeners: true,
                    skip_io_mappings: true,
                },
            )
            .await
            .expect("телепорт");

        let requests = server.received_requests().await.unwrap_or_default();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "POST");
        assert_eq!(requests[0].url.path(), "/engine-rest/process-instance/instance-1/modification");
        let payload = body_json(&requests[0]);
        assert_eq!(payload["annotation"], "INC-42");
        assert_eq!(payload["skipCustomListeners"], true);
        assert_eq!(payload["skipIoMappings"], true);
        let instructions = payload["instructions"].as_array().expect("инструкции");
        assert_eq!(instructions[0]["type"], "cancel");
        assert_eq!(instructions[0]["activityInstanceId"], "activity-instance-7");
        assert_eq!(instructions[1]["type"], "startBeforeActivity");
        assert_eq!(instructions[1]["activityId"], "Activity_target");
    }
}
