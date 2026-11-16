use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::Value;

use super::ApiResult;
use super::activity_history::activity_sort_name;
use super::json::{as_array, json_object, string, string_array};
use crate::models::*;

// Текущие активности: runtime-дерево, дополненное признаками из инцидентов, job, external task и незакрытой истории.
pub(super) fn build_current_activities(
    runtime_activities: Vec<ActiveActivityInstance>,
    activity_history: &[ActivityExecutionSummary],
    incidents: &[ProcessIncident],
    jobs: &[ProcessJob],
    external_tasks: &[ProcessExternalTask],
    diagram: &BpmnDiagram,
) -> Vec<ActiveActivityInstance> {
    let runtime_activity_ids: HashSet<String> = runtime_activities.iter().map(|activity| activity.activity_id.clone()).collect();
    let mut evidence_by_activity_id: HashMap<String, BTreeSet<CurrentActivityEvidence>> = HashMap::new();
    let mut add_evidence = |activity_id: Option<&str>, source: CurrentActivityEvidence| {
        if let Some(activity_id) = activity_id.filter(|id| !id.trim().is_empty()) {
            evidence_by_activity_id.entry(activity_id.to_string()).or_default().insert(source);
        }
    };
    for incident in incidents {
        add_evidence(incident_activity_id(incident), CurrentActivityEvidence::Incident);
    }
    for job in jobs {
        add_evidence(job.failed_activity_id.as_deref(), CurrentActivityEvidence::Job);
    }
    for task in external_tasks {
        add_evidence(Some(task.activity_id.as_str()), CurrentActivityEvidence::ExternalTask);
    }
    for summary in
        activity_history.iter().filter(|summary| summary.active_count > 0 && !runtime_activity_ids.contains(&summary.activity_id))
    {
        add_evidence(Some(summary.activity_id.as_str()), CurrentActivityEvidence::UnfinishedHistory);
    }

    let mut activities: Vec<ActiveActivityInstance> = runtime_activities
        .into_iter()
        .map(|mut activity| {
            if let Some(extra) = evidence_by_activity_id.get(&activity.activity_id) {
                let mut evidence: BTreeSet<CurrentActivityEvidence> = activity.evidence.iter().copied().collect();
                evidence.extend(extra.iter().copied());
                activity.evidence = evidence.into_iter().collect();
            }
            activity
        })
        .collect();
    activities.extend(evidence_by_activity_id.iter().filter(|(activity_id, _)| !runtime_activity_ids.contains(*activity_id)).map(
        |(activity_id, evidence)| {
            let node = diagram.node(activity_id);
            ActiveActivityInstance {
                id: format!("diagnostic:{activity_id}"),
                activity_id: activity_id.clone(),
                activity_name: node.and_then(|node| node.name.clone()),
                activity_type: node.map(|node| node.node_type.clone()).unwrap_or_else(|| "unknown".to_string()),
                execution_ids: Vec::new(),
                incident_ids: incidents
                    .iter()
                    .filter(|incident| incident_activity_id(incident) == Some(activity_id.as_str()))
                    .map(|incident| incident.id.clone())
                    .collect(),
                evidence: evidence.iter().copied().collect(),
                cancellable: false,
            }
        },
    ));
    activities
        .sort_by_cached_key(|activity| (activity_sort_name(activity.activity_name.as_deref(), &activity.activity_id), activity.id.clone()));
    activities
}

pub(super) fn parse_activity_tree(body: &str) -> ApiResult<Vec<ActiveActivityInstance>> {
    let mut result = Vec::new();
    visit_activity(&json_object(body)?, true, &mut result);
    Ok(result)
}

fn visit_activity(value: &Value, root: bool, result: &mut Vec<ActiveActivityInstance>) {
    let activity_type = string(value, "activityType").unwrap_or_else(|| "unknown".to_string());
    if !root && activity_type != "processDefinition" {
        result.push(ActiveActivityInstance {
            id: string(value, "id").unwrap_or_default(),
            activity_id: string(value, "activityId").unwrap_or_default(),
            activity_name: string(value, "activityName").or_else(|| string(value, "name")),
            activity_type,
            execution_ids: string_array(value, "executionIds"),
            incident_ids: string_array(value, "incidentIds"),
            evidence: vec![CurrentActivityEvidence::RuntimeActivity],
            cancellable: true,
        });
    }
    for child in as_array(value.get("childActivityInstances")) {
        visit_activity(child, false, result);
    }
    for transition in as_array(value.get("childTransitionInstances")) {
        result.push(ActiveActivityInstance {
            id: string(transition, "id").unwrap_or_default(),
            activity_id: string(transition, "activityId").unwrap_or_default(),
            activity_name: string(transition, "activityName").or_else(|| string(transition, "name")),
            activity_type: string(transition, "activityType").unwrap_or_else(|| "transition".to_string()),
            execution_ids: string(transition, "executionId").into_iter().collect(),
            incident_ids: string_array(transition, "incidentIds"),
            evidence: vec![CurrentActivityEvidence::Transition],
            cancellable: false,
        });
    }
}

fn incident_activity_id(incident: &ProcessIncident) -> Option<&str> {
    incident.failed_activity_id.as_deref().or(incident.activity_id.as_deref())
}
