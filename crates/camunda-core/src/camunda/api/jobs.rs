use super::ApiResult;
use super::json::{boolean, int, json_array, nullable_int, string};
use crate::models::*;

pub(super) fn parse_jobs(body: &str) -> ApiResult<Vec<ProcessJob>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(ProcessJob {
                id: string(value, "id")?,
                retries: int(value, "retries"),
                failed_activity_id: string(value, "failedActivityId"),
                exception_message: string(value, "exceptionMessage"),
                due_date: string(value, "dueDate"),
                suspended: boolean(value, "suspended"),
            })
        })
        .collect())
}

pub(super) fn parse_external_tasks(body: &str) -> ApiResult<Vec<ProcessExternalTask>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(ProcessExternalTask {
                id: string(value, "id")?,
                activity_id: string(value, "activityId").unwrap_or_default(),
                topic_name: string(value, "topicName").unwrap_or_default(),
                retries: nullable_int(value, "retries"),
                error_message: string(value, "errorMessage"),
                worker_id: string(value, "workerId"),
                lock_expiration_time: string(value, "lockExpirationTime"),
                suspended: boolean(value, "suspended"),
            })
        })
        .collect())
}
