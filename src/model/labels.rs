use camunda_core::models::ProcessDefinitionSummary;

// Имя процесса, а если оно пустое — ключ.
pub fn display_name(definition: &ProcessDefinitionSummary) -> &str {
    definition.name.as_deref().filter(|name| !name.trim().is_empty()).unwrap_or(&definition.key)
}

pub fn incident_type_label(incident_type: &str) -> &str {
    match incident_type {
        "failedJob" => "Ошибка задания",
        "failedExternalTask" => "Ошибка внешней задачи",
        "" => "Неизвестный тип",
        other => other,
    }
}
