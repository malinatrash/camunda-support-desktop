use super::InstanceView;

impl InstanceView {
    // Markdown-отчёт о состоянии заявки для обращения или чата.
    pub(super) fn report(&self) -> String {
        let Some(details) = &self.details else { return String::new() };
        let instance = &details.instance;
        let state = if instance.ended {
            "завершена"
        } else if instance.suspended {
            "приостановлена"
        } else {
            "запущена"
        };
        let mut lines = vec![
            format!("# Заявка {}", instance.business_key.as_deref().unwrap_or(&instance.id)),
            format!("- Подключение: {} ({})", self.connection.name, self.connection.rest_url),
            format!("- Процесс: {} · {}", instance.definition_key, instance.definition_id),
            format!("- ID заявки: {}", instance.id),
            format!("- Состояние: {state}"),
            String::new(),
            "## Текущие позиции".into(),
        ];
        lines.extend(
            details
                .active_activities
                .iter()
                .map(|a| format!("- {} ({})", a.activity_name.as_deref().unwrap_or(&a.activity_id), a.activity_id)),
        );
        lines.push(String::new());
        lines.push(format!("## Инциденты ({})", details.incidents.len()));
        for incident in &details.incidents {
            lines.push(format!(
                "- {} · {} · {}",
                incident.incident_type,
                incident.failed_activity_id.as_deref().or(incident.activity_id.as_deref()).unwrap_or("—"),
                incident.timestamp.as_deref().unwrap_or("")
            ));
            if let Some(message) = &incident.message {
                lines.push(format!("  ```\n  {}\n  ```", message.replace('\n', "\n  ")));
            }
        }
        lines.push(String::new());
        lines.push("## Переменные".into());
        lines.extend(details.variables.iter().map(|v| {
            let value: String = v.value.chars().take(200).collect();
            format!("- {} ({}): {value}", v.name, v.value_type)
        }));
        lines.join("\n")
    }
}
