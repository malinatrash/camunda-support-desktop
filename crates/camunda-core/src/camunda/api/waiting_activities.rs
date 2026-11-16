use std::collections::HashSet;

use chrono::Utc;

use super::activity_history::{HistoricActivityInstance, parse_historic_activity_instances};
use super::grouping::group_in_order;
use super::{ApiResult, CamundaApi};
use crate::camunda::dates;
use crate::models::*;

impl CamundaApi {
    pub(super) async fn load_unfinished_activities(&self, connection: &CamundaConnection) -> ApiResult<Vec<HistoricActivityInstance>> {
        let base_url = format!("{}/history/activity-instance?unfinished=true&sortBy=startTime&sortOrder=asc", connection.rest_url);
        self.load_paged(&base_url, parse_historic_activity_instances).await
    }

    // Среди незавершённых активностей выбирает листовые (не родители) и считает ожидание до текущего момента.
    pub(super) async fn load_longest_waiting_activities(
        &self,
        connection: &CamundaConnection,
        activities: &[&HistoricActivityInstance],
    ) -> Vec<DashboardWaitingActivity> {
        let parent_ids: HashSet<&str> = activities.iter().filter_map(|activity| activity.parent_activity_instance_id.as_deref()).collect();
        let current = (self.now)();
        let samples = activities
            .iter()
            .filter(|activity| !parent_ids.contains(activity.id.as_str()) && activity.activity_type != "processDefinition")
            .filter_map(|activity| {
                activity.process_definition_id.as_deref()?;
                let process_instance_id = activity.process_instance_id.as_deref()?;
                let start = dates::parse_camunda_date_time(activity.start_time.as_deref())?;
                let waiting_millis = (current - start.with_timezone(&Utc)).num_milliseconds().max(0);
                Some(WaitingActivitySample { activity, process_instance_id, waiting_millis })
            });
        let grouped = group_in_order(samples, |sample| {
            (sample.activity.process_definition_id.clone().unwrap_or_default(), sample.activity.activity_id.clone())
        });
        let mut aggregated: Vec<DashboardWaitingActivity> = grouped
            .into_iter()
            .filter_map(|((process_definition_id, activity_id), samples)| {
                let oldest = samples.iter().rev().max_by_key(|sample| sample.waiting_millis)?;
                let instance_ids: HashSet<&str> = samples.iter().map(|sample| sample.process_instance_id).collect();
                let total_waiting: i64 = samples.iter().map(|sample| sample.waiting_millis).sum();
                Some(DashboardWaitingActivity {
                    activity_id,
                    activity_name: samples
                        .iter()
                        .find_map(|sample| sample.activity.activity_name.as_deref().filter(|name| !name.trim().is_empty()))
                        .map(str::to_string),
                    activity_type: oldest.activity.activity_type.clone(),
                    topic: None,
                    process_definition_id,
                    oldest_process_instance_id: oldest.process_instance_id.to_string(),
                    instance_count: instance_ids.len() as i32,
                    average_waiting_millis: (total_waiting as f64 / samples.len() as f64) as i64,
                    maximum_waiting_millis: oldest.waiting_millis,
                    oldest_started_at: oldest.activity.start_time.clone(),
                })
            })
            .collect();
        aggregated.sort_by_key(|item| std::cmp::Reverse(item.maximum_waiting_millis));
        aggregated.truncate(DASHBOARD_WAITING_ACTIVITY_LIMIT);

        let diagrams =
            self.load_diagrams(connection, aggregated.iter().map(|activity| activity.process_definition_id.clone()).collect()).await;
        aggregated
            .into_iter()
            .map(|activity| {
                let node = diagrams.get(&activity.process_definition_id).and_then(|loaded| loaded.diagram.node(&activity.activity_id));
                DashboardWaitingActivity {
                    activity_name: activity.activity_name.or_else(|| node.and_then(|node| node.name.clone())),
                    topic: node.and_then(|node| node.topic.clone()),
                    ..activity
                }
            })
            .collect()
    }
}

struct WaitingActivitySample<'a> {
    activity: &'a HistoricActivityInstance,
    process_instance_id: &'a str,
    waiting_millis: i64,
}

const DASHBOARD_WAITING_ACTIVITY_LIMIT: usize = 6;
