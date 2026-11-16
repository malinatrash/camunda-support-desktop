use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: String,
    pub value_info: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableValueType {
    Auto,
    String,
    Number,
    Boolean,
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
