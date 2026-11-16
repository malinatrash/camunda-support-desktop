use serde_json::Value;

use super::json::string;
use crate::models::*;

pub(super) fn definition_metadata(value: &Value) -> Vec<ProcessMetadataField> {
    vec![
        metadata_field(&rust_i18n::t!("core.metadata.definition_id"), value, "id"),
        metadata_field(&rust_i18n::t!("core.metadata.key"), value, "key"),
        metadata_field(&rust_i18n::t!("core.metadata.name"), value, "name"),
        metadata_field(&rust_i18n::t!("core.metadata.description"), value, "description"),
        metadata_field(&rust_i18n::t!("core.metadata.version"), value, "version"),
        metadata_field(&rust_i18n::t!("core.metadata.version_tag"), value, "versionTag"),
        metadata_field(&rust_i18n::t!("core.metadata.category"), value, "category"),
        metadata_field(&rust_i18n::t!("core.metadata.bpmn_resource"), value, "resource"),
        metadata_field(&rust_i18n::t!("core.metadata.diagram_resource"), value, "diagram"),
        metadata_field(&rust_i18n::t!("core.metadata.deployment_id"), value, "deploymentId"),
        metadata_field(&rust_i18n::t!("core.metadata.tenant_id"), value, "tenantId"),
        metadata_field(&rust_i18n::t!("core.metadata.history_ttl"), value, "historyTimeToLive"),
        metadata_field(&rust_i18n::t!("core.metadata.startable_in_tasklist"), value, "startableInTasklist"),
        metadata_field(&rust_i18n::t!("core.metadata.suspended"), value, "suspended"),
    ]
}

pub(super) fn instance_metadata(value: &Value) -> Vec<ProcessMetadataField> {
    vec![
        metadata_field(&rust_i18n::t!("core.metadata.instance_id"), value, "id"),
        metadata_field(&rust_i18n::t!("core.metadata.definition_id"), value, "definitionId"),
        metadata_field(&rust_i18n::t!("core.metadata.definition_key"), value, "definitionKey"),
        metadata_field(&rust_i18n::t!("core.metadata.business_key"), value, "businessKey"),
        metadata_field(&rust_i18n::t!("core.metadata.case_instance_id"), value, "caseInstanceId"),
        metadata_field(&rust_i18n::t!("core.metadata.tenant_id"), value, "tenantId"),
        metadata_field(&rust_i18n::t!("core.metadata.ended"), value, "ended"),
        metadata_field(&rust_i18n::t!("core.metadata.suspended"), value, "suspended"),
    ]
}

fn metadata_field(label: &str, value: &Value, key: &str) -> ProcessMetadataField {
    ProcessMetadataField { label: label.to_string(), value: string(value, key).unwrap_or_else(|| "—".to_string()) }
}
