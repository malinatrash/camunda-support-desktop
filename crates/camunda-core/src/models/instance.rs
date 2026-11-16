use serde::{Deserialize, Serialize};

use super::{BpmnDiagram, ProcessExternalTask, ProcessIncident, ProcessJob, ProcessMetadataField, ProcessVariable};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInstanceSummary {
    pub id: String,
    pub definition_id: String,
    pub definition_key: String,
    pub business_key: Option<String>,
    pub suspended: bool,
    pub tenant_id: Option<String>,
    pub ended: bool,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub state: Option<String>,
    #[serde(default)]
    pub definition_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInstanceDetails {
    pub instance: ProcessInstanceSummary,
    pub variables: Vec<ProcessVariable>,
    pub active_activities: Vec<ActiveActivityInstance>,
    pub activity_history: Vec<ActivityExecutionSummary>,
    pub incidents: Vec<ProcessIncident>,
    pub jobs: Vec<ProcessJob>,
    pub external_tasks: Vec<ProcessExternalTask>,
    pub diagram: BpmnDiagram,
    pub bpmn_xml: String,
    pub metadata: Vec<ProcessMetadataField>,
    #[serde(default)]
    pub activity_timeline: Vec<ActivityTimelineEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityTimelineEntry {
    pub id: String,
    pub activity_id: String,
    pub activity_name: Option<String>,
    pub activity_type: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration_millis: Option<i64>,
    pub canceled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityExecutionSummary {
    pub activity_id: String,
    pub activity_name: Option<String>,
    pub activity_type: String,
    pub completed_count: i32,
    pub active_count: i32,
    pub canceled_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CurrentActivityEvidence {
    RuntimeActivity,
    Transition,
    UnfinishedHistory,
    Incident,
    Job,
    ExternalTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveActivityInstance {
    pub id: String,
    pub activity_id: String,
    pub activity_name: Option<String>,
    pub activity_type: String,
    pub execution_ids: Vec<String>,
    pub incident_ids: Vec<String>,
    pub evidence: Vec<CurrentActivityEvidence>,
    pub cancellable: bool,
}
