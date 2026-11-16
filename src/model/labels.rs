use camunda_core::models::ProcessDefinitionSummary;
use gpui_kit::SharedString;

// Имя процесса, а если оно пустое — ключ.
pub fn display_name(definition: &ProcessDefinitionSummary) -> &str {
    definition.name.as_deref().filter(|name| !name.trim().is_empty()).unwrap_or(&definition.key)
}

pub fn incident_type_label(incident_type: &str) -> SharedString {
    match incident_type {
        "failedJob" => tr!("model.incident_type.failed_job"),
        "failedExternalTask" => tr!("model.incident_type.failed_external_task"),
        "" => tr!("model.incident_type.unknown"),
        other => SharedString::from(other.to_string()),
    }
}
