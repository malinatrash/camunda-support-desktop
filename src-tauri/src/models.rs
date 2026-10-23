use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    Production,
    Test,
    Development,
    Local,
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Test
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CamundaConnection {
    pub id: String,
    pub name: String,
    pub rest_url: String,
    pub cockpit_url: String,
    pub environment: Environment,
    #[serde(default)]
    pub engine_version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub rest_url: String,
    #[serde(default)]
    pub cockpit_url: String,
    #[serde(default = "default_environment")]
    pub environment: Environment,
}

fn default_environment() -> Environment {
    Environment::Test
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionValidation {
    pub name_error: Option<String>,
    pub rest_url_error: Option<String>,
    pub cockpit_url_error: Option<String>,
}

impl ConnectionValidation {
    pub fn is_valid(&self) -> bool {
        self.name_error.is_none() && self.rest_url_error.is_none() && self.cockpit_url_error.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableValueType {
    Auto,
    String,
    Number,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DashboardDatePreset {
    Live,
    Today,
    Yesterday,
    Custom,
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
pub struct ProcessDashboard {
    pub definitions: Vec<ProcessDefinitionSummary>,
    pub timeline: Vec<DashboardTimelinePoint>,
    pub incident_types: Vec<DashboardIncidentType>,
    pub incident_activities: Vec<DashboardIncidentActivity>,
    pub longest_waiting_activities: Vec<DashboardWaitingActivity>,
    pub duration_buckets: Vec<DashboardDurationBucket>,
    pub timeline_title: String,
    pub average_duration_millis: Option<i64>,
    pub median_duration_millis: Option<i64>,
    pub p95_duration_millis: Option<i64>,
    pub completed_instances: i32,
    pub timeline_truncated: bool,
    #[serde(default)]
    pub incidents: Vec<ProcessIncident>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardTimelinePoint {
    pub label: String,
    pub started: i32,
    pub completed: i32,
    pub incidents: i32,
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
pub struct DashboardDurationBucket {
    pub label: String,
    pub count: i32,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRetryResult {
    pub jobs: i32,
    pub external_tasks: i32,
    pub skipped: i32,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMetadataField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: String,
    pub value_info: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessVariableCatalog {
    pub process_definition_key: String,
    pub variables: Vec<ProcessVariableDescriptor>,
    pub inspected_instance_count: i32,
    pub instances_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessVariableDescriptor {
    pub name: String,
    pub types: Vec<String>,
    pub occurrences: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessVariableUpdate {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: String,
    pub value_info: Option<String>,
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

pub const TELEPORT_TARGET_TYPES: &[&str] = &[
    "task",
    "userTask",
    "serviceTask",
    "scriptTask",
    "manualTask",
    "businessRuleTask",
    "sendTask",
    "receiveTask",
    "callActivity",
    "subProcess",
    "intermediateCatchEvent",
    "intermediateThrowEvent",
    "exclusiveGateway",
    "parallelGateway",
    "inclusiveGateway",
    "eventBasedGateway",
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BpmnDiagram {
    pub nodes: Vec<BpmnNode>,
    pub edges: Vec<BpmnEdge>,
}

impl BpmnDiagram {
    pub fn teleport_targets(&self) -> impl Iterator<Item = &BpmnNode> {
        self.nodes.iter().filter(|node| TELEPORT_TARGET_TYPES.contains(&node.node_type.as_str()))
    }

    pub fn node(&self, id: &str) -> Option<&BpmnNode> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BpmnNode {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub node_type: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BpmnEdge {
    pub id: String,
    pub source_ref: Option<String>,
    pub target_ref: Option<String>,
    pub waypoints: Vec<BpmnPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BpmnPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeleportRequest {
    pub source_activity_instance_id: String,
    pub target_activity_id: String,
    pub annotation: String,
    #[serde(default = "default_true")]
    pub skip_custom_listeners: bool,
    #[serde(default = "default_true")]
    pub skip_io_mappings: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestSuccess {
    pub engine_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateAsset {
    pub name: String,
    pub download_url: String,
    pub size_bytes: i64,
    pub digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    pub version: String,
    pub tag: String,
    pub release_url: String,
    pub notes: String,
    pub asset: Option<AppUpdateAsset>,
    pub checksum_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateCheckResult {
    UpToDate,
    Available { update: AppUpdate },
    Failure { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateInstallResult {
    InstallerOpened { file_name: String },
    Failure { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateDownloadStage {
    Preparing,
    Downloading,
    Verifying,
    OpeningInstaller,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDownloadProgress {
    pub stage: UpdateDownloadStage,
    pub downloaded_bytes: i64,
    pub total_bytes: i64,
    pub bytes_per_second: i64,
    pub destination_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseDownloadStats {
    pub total_downloads: i64,
    pub mac_os_downloads: i64,
    pub windows_downloads: i64,
    pub linux_downloads: i64,
    pub release_count: i32,
    pub installer_asset_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInstanceDeepLink {
    pub origin: String,
    pub process_instance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeepLinkEvent {
    OpenProcessInstance { link: ProcessInstanceDeepLink },
    Invalid { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub build: String,
    pub platform: String,
}
