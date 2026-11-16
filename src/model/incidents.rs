use std::collections::HashMap;

use camunda_core::models::ProcessIncident;
use gpui_kit::SharedString;

use super::signature::incident_signature;

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
                activity: if activity.is_empty() { tr!("model.no_activity") } else { activity.into() },
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
