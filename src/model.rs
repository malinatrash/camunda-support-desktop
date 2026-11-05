use std::collections::HashMap;

use camunda_core::models::{ProcessDefinitionSummary, ProcessIncident};
use gpui_kit::SharedString;

pub struct IncidentGroup {
    pub key: SharedString,
    pub activity: SharedString,
    pub signature: SharedString,
    pub process_key: SharedString,
    pub incident_type: String,
    pub count: usize,
    pub instance_ids: Vec<String>,
    pub latest: Option<String>,
}

pub struct ProcessGroup {
    pub key: SharedString,
    pub latest: ProcessDefinitionSummary,
    pub versions: Vec<ProcessDefinitionSummary>,
    pub instances: i32,
    pub incidents: i32,
}

// Группирует инциденты по процессу, кубику, типу и подписи сообщения; крупные группы первыми.
pub fn group_incidents(incidents: &[ProcessIncident]) -> Vec<IncidentGroup> {
    let mut groups: Vec<IncidentGroup> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for incident in incidents {
        let activity = incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_default();
        let process_key = incident.process_definition_id.as_deref().and_then(|id| id.split(':').next()).unwrap_or_default().to_string();
        let signature = incident_signature(incident.message.as_deref());
        let key = format!("{process_key}|{activity}|{}|{signature}", incident.incident_type);
        let position = *index.entry(key.clone()).or_insert_with(|| {
            groups.push(IncidentGroup {
                key: key.into(),
                activity: if activity.is_empty() { "Без кубика".into() } else { activity.into() },
                signature: signature.into(),
                process_key: process_key.into(),
                incident_type: incident.incident_type.clone(),
                count: 0,
                instance_ids: Vec::new(),
                latest: None,
            });
            groups.len() - 1
        });
        let group = &mut groups[position];
        group.count += 1;
        if let Some(id) = &incident.process_instance_id
            && !group.instance_ids.contains(id)
        {
            group.instance_ids.push(id.clone());
        }
        if incident.timestamp > group.latest {
            group.latest = incident.timestamp.clone();
        }
    }
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| b.latest.cmp(&a.latest)));
    groups
}

// Сворачивает версии в процесс по ключу и тенанту; версии внутри по дате деплоя, сначала новые.
pub fn group_processes(definitions: &[ProcessDefinitionSummary]) -> Vec<ProcessGroup> {
    let mut by_key: HashMap<String, Vec<ProcessDefinitionSummary>> = HashMap::new();
    for definition in definitions {
        let key = format!("{}|{}", definition.key, definition.tenant_id.as_deref().unwrap_or_default());
        by_key.entry(key).or_default().push(definition.clone());
    }
    let mut groups: Vec<ProcessGroup> = by_key
        .into_iter()
        .map(|(key, mut versions)| {
            versions.sort_by(|a, b| b.deployment_time.cmp(&a.deployment_time).then(b.version.cmp(&a.version)));
            let latest = versions.iter().max_by_key(|d| d.version).cloned().expect("непустая группа");
            ProcessGroup {
                key: key.into(),
                instances: versions.iter().map(|d| d.instances).sum(),
                incidents: versions.iter().map(|d| d.incidents).sum(),
                latest,
                versions,
            }
        })
        .collect();
    groups.sort_by(|a, b| {
        b.incidents.cmp(&a.incidents).then(b.instances.cmp(&a.instances)).then_with(|| display_name(&a.latest).cmp(display_name(&b.latest)))
    });
    groups
}

pub fn display_name(definition: &ProcessDefinitionSummary) -> &str {
    definition.name.as_deref().filter(|name| !name.trim().is_empty()).unwrap_or(&definition.key)
}

// Подпись сообщения без идентификаторов и чисел: первая непустая строка, до 140 символов.
pub fn incident_signature(message: Option<&str>) -> String {
    let Some(message) = message else { return "Без сообщения".to_string() };
    let line = message.lines().find(|line| !line.trim().is_empty()).unwrap_or(message);
    let words: Vec<String> = line
        .split_whitespace()
        .map(|word| {
            let hexish = word.len() >= 16 && word.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
            if hexish { "#".to_string() } else { collapse_digits(word) }
        })
        .collect();
    words.join(" ").chars().take(140).collect()
}

fn collapse_digits(word: &str) -> String {
    let mut result = String::with_capacity(word.len());
    let mut in_digits = false;
    for c in word.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                result.push('#');
            }
            in_digits = true;
        } else {
            result.push(c);
            in_digits = false;
        }
    }
    result
}

pub fn incident_type_label(incident_type: &str) -> &str {
    match incident_type {
        "failedJob" => "Ошибка задания",
        "failedExternalTask" => "Ошибка внешней задачи",
        "" => "Неизвестный тип",
        other => other,
    }
}

pub fn grouped(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut result = String::new();
    for (index, c) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            result.push('\u{202f}');
        }
        result.push(c);
    }
    if value < 0 { format!("-{result}") } else { result }
}

pub fn format_duration(millis: i64) -> String {
    let minutes = millis / 60_000;
    let (days, hours, rest) = (minutes / (24 * 60), (minutes % (24 * 60)) / 60, minutes % 60);
    if days > 0 {
        format!("{days} д. {hours} ч.")
    } else if hours > 0 {
        format!("{hours} ч. {rest} мин.")
    } else {
        format!("{rest} мин.")
    }
}

// «2026-10-30 12:00» из метки времени Camunda.
pub fn short_time(timestamp: Option<&str>) -> String {
    timestamp.map(|value| value.chars().take(16).collect::<String>().replace('T', " ")).unwrap_or_else(|| "—".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_hides_ids_and_numbers() {
        let message = "Read timed out after 3000 ms for 9d1f6e2a-0001-4bcd-8ef0-123456789abc\nstack";
        assert_eq!(incident_signature(Some(message)), "Read timed out after # ms for #");
    }

    #[test]
    fn grouped_splits_thousands() {
        assert_eq!(grouped(1421), "1\u{202f}421");
        assert_eq!(grouped(999), "999");
    }
}
