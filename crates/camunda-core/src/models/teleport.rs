use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeleportRequest {
    pub source_activity_instance_id: String,
    pub target_activity_id: String,
    pub annotation: String,
    #[serde(default = "super::default_true")]
    pub skip_custom_listeners: bool,
    #[serde(default = "super::default_true")]
    pub skip_io_mappings: bool,
}
