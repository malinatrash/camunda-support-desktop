use serde::{Deserialize, Serialize};

use super::{DashboardDateFilter, ProcessInstanceSummary};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDefinitionSummary {
    pub id: String,
    pub key: String,
    pub name: Option<String>,
    pub version: i32,
    pub tenant_id: Option<String>,
    pub suspended: bool,
    pub deployment_id: Option<String>,
    pub deployment_time: Option<String>,
    pub instances: i32,
    pub incidents: i32,
    pub version_tag: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub resource: Option<String>,
    pub diagram_resource: Option<String>,
    pub history_time_to_live: Option<i32>,
    pub startable_in_tasklist: bool,
}

impl ProcessDefinitionSummary {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().filter(|name| !name.trim().is_empty()).unwrap_or(&self.key)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDefinitionDetails {
    pub definition: ProcessDefinitionSummary,
    pub instances: Vec<RuntimeInstanceListItem>,
    pub metadata: Vec<ProcessMetadataField>,
    pub date_filter: DashboardDateFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstanceListItem {
    pub instance: ProcessInstanceSummary,
    pub incident_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMetadataField {
    pub label: String,
    pub value: String,
}
