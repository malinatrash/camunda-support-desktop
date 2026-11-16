use serde::{Deserialize, Serialize};

use super::{ProcessDefinitionSummary, ProcessIncident};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDashboard {
    pub definitions: Vec<ProcessDefinitionSummary>,
    pub incident_types: Vec<DashboardIncidentType>,
    pub incident_activities: Vec<DashboardIncidentActivity>,
    pub longest_waiting_activities: Vec<DashboardWaitingActivity>,
    pub completed_instances: i32,
    #[serde(default)]
    pub incidents: Vec<ProcessIncident>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDateFilter {
    #[serde(default = "default_preset")]
    pub preset: DashboardDatePreset,
    #[serde(default)]
    pub from_date: String,
    #[serde(default)]
    pub to_date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DashboardDatePreset {
    Live,
    Today,
    Yesterday,
    Custom,
}

fn default_preset() -> DashboardDatePreset {
    DashboardDatePreset::Live
}

impl Default for DashboardDateFilter {
    fn default() -> Self {
        Self { preset: DashboardDatePreset::Live, from_date: String::new(), to_date: String::new() }
    }
}

impl DashboardDateFilter {
    pub fn is_live(&self) -> bool {
        self.preset == DashboardDatePreset::Live
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardIncidentType {
    #[serde(rename = "type")]
    pub incident_type: String,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardIncidentActivity {
    pub activity_id: String,
    pub process_definition_id: Option<String>,
    pub count: i32,
    pub activity_name: Option<String>,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardWaitingActivity {
    pub activity_id: String,
    pub activity_name: Option<String>,
    pub activity_type: String,
    pub topic: Option<String>,
    pub process_definition_id: String,
    pub oldest_process_instance_id: String,
    pub instance_count: i32,
    pub average_waiting_millis: i64,
    pub maximum_waiting_millis: i64,
    pub oldest_started_at: Option<String>,
}
