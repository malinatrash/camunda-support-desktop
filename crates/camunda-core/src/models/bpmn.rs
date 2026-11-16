use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub event_definition: Option<String>,
    #[serde(default = "super::default_true")]
    pub interrupting: bool,
    #[serde(default)]
    pub loop_marker: Option<LoopMarker>,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default)]
    pub triggered_by_event: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMarker {
    Parallel,
    Sequential,
    Standard,
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
