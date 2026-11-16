use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeepLinkEvent {
    OpenProcessInstance { link: ProcessInstanceDeepLink },
    Invalid { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInstanceDeepLink {
    pub origin: String,
    pub process_instance_id: String,
}
