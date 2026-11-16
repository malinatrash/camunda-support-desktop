use std::collections::HashMap;

use camunda_core::models::ProcessDefinitionSummary;
use gpui_kit::SharedString;

use super::labels::display_name;

pub struct ProcessGroup {
    pub key: SharedString,
    pub latest: ProcessDefinitionSummary,
    pub versions: Vec<ProcessDefinitionSummary>,
    pub instances: i32,
    pub incidents: i32,
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
