use super::ApiResult;
use super::grouping::group_in_order;
use super::json::{boolean, json_array, string};
use crate::camunda::dates;
use crate::models::*;

#[derive(Debug, Clone)]
pub(super) struct HistoricActivityInstance {
    pub(super) id: String,
    pub(super) activity_id: String,
    pub(super) activity_name: Option<String>,
    pub(super) activity_type: String,
    pub(super) process_definition_id: Option<String>,
    pub(super) process_instance_id: Option<String>,
    pub(super) parent_activity_instance_id: Option<String>,
    pub(super) start_time: Option<String>,
    pub(super) end_time: Option<String>,
    pub(super) canceled: bool,
}

pub(super) fn parse_historic_activity_instances(body: &str) -> ApiResult<Vec<HistoricActivityInstance>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(HistoricActivityInstance {
                id: string(value, "id")?,
                activity_id: string(value, "activityId")?,
                activity_name: string(value, "activityName"),
                activity_type: string(value, "activityType").unwrap_or_else(|| "unknown".to_string()),
                process_definition_id: string(value, "processDefinitionId"),
                process_instance_id: string(value, "processInstanceId"),
                parent_activity_instance_id: string(value, "parentActivityInstanceId"),
                start_time: string(value, "startTime"),
                end_time: string(value, "endTime"),
                canceled: boolean(value, "canceled"),
            })
        })
        .collect())
}

pub(super) fn to_activity_timeline(history: &[HistoricActivityInstance]) -> Vec<ActivityTimelineEntry> {
    history
        .iter()
        .filter(|activity| activity.activity_type != "processDefinition")
        .map(|activity| ActivityTimelineEntry {
            id: activity.id.clone(),
            activity_id: activity.activity_id.clone(),
            activity_name: activity.activity_name.clone(),
            activity_type: activity.activity_type.clone(),
            start_time: activity.start_time.clone(),
            end_time: activity.end_time.clone(),
            duration_millis: dates::duration_millis(activity.start_time.as_deref(), activity.end_time.as_deref()),
            canceled: activity.canceled,
        })
        .collect()
}

pub(super) fn to_activity_execution_summaries(history: &[HistoricActivityInstance]) -> Vec<ActivityExecutionSummary> {
    let grouped = group_in_order(history.iter().filter(|activity| activity.activity_type != "processDefinition"), |activity| {
        activity.activity_id.clone()
    });
    let mut summaries: Vec<ActivityExecutionSummary> = grouped
        .into_iter()
        .filter_map(|(activity_id, executions)| {
            let representative = executions.last()?;
            Some(ActivityExecutionSummary {
                activity_id,
                activity_name: executions
                    .iter()
                    .rev()
                    .find_map(|execution| execution.activity_name.as_deref().filter(|name| !name.trim().is_empty()))
                    .map(str::to_string),
                activity_type: representative.activity_type.clone(),
                completed_count: executions.iter().filter(|execution| execution.end_time.is_some() && !execution.canceled).count() as i32,
                active_count: executions.iter().filter(|execution| execution.end_time.is_none()).count() as i32,
                canceled_count: executions.iter().filter(|execution| execution.canceled).count() as i32,
            })
        })
        .collect();
    summaries.sort_by_cached_key(|summary| activity_sort_name(summary.activity_name.as_deref(), &summary.activity_id));
    summaries
}

pub(super) fn activity_sort_name(activity_name: Option<&str>, activity_id: &str) -> String {
    activity_name.map(str::to_lowercase).unwrap_or_else(|| activity_id.to_lowercase())
}
