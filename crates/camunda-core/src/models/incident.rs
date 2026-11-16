use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessIncident {
    pub id: String,
    #[serde(rename = "type")]
    pub incident_type: String,
    pub activity_id: Option<String>,
    pub failed_activity_id: Option<String>,
    pub message: Option<String>,
    pub timestamp: Option<String>,
    pub configuration: Option<String>,
    pub process_definition_id: Option<String>,
    pub process_instance_id: Option<String>,
    pub execution_id: Option<String>,
    pub tenant_id: Option<String>,
    pub annotation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessJob {
    pub id: String,
    pub retries: i32,
    pub failed_activity_id: Option<String>,
    pub exception_message: Option<String>,
    pub due_date: Option<String>,
    pub suspended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessExternalTask {
    pub id: String,
    pub activity_id: String,
    pub topic_name: String,
    pub retries: Option<i32>,
    pub error_message: Option<String>,
    pub worker_id: Option<String>,
    pub lock_expiration_time: Option<String>,
    pub suspended: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRetryResult {
    pub jobs: i32,
    pub external_tasks: i32,
    pub skipped: i32,
}
