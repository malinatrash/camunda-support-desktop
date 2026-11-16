use super::{LegacyNodes, LegacyPreferences};
use crate::connection::{connection_id, derive_cockpit_url};
use crate::models::{AppThemeMode, CamundaConnection, Environment};

// Собирает подключения из сырых узлов: без name/restUrl пропускает, id генерирует заново, сортирует по имени.
pub fn build_legacy_preferences(nodes: LegacyNodes) -> LegacyPreferences {
    let mut connections: Vec<CamundaConnection> = Vec::new();
    for values in nodes.connections {
        let value = |key: &str| values.get(key).map(|value| value.trim()).unwrap_or_default();
        let name = value("name");
        let rest_url = value("restUrl").trim_end_matches('/').to_string();
        if name.is_empty() || rest_url.is_empty() {
            continue;
        }
        let cockpit_url = value("cockpitUrl").trim_end_matches('/').to_string();
        connections.push(CamundaConnection {
            id: connection_id(name, &connections),
            name: name.to_string(),
            cockpit_url: if cockpit_url.is_empty() { derive_cockpit_url(&rest_url) } else { cockpit_url },
            rest_url,
            environment: parse_environment(value("environment")),
            engine_version: Some(value("engineVersion").to_string()).filter(|version| !version.is_empty()),
        });
    }
    connections.sort_by_key(|connection| connection.name.to_lowercase());
    let theme = nodes.theme.as_deref().and_then(parse_theme);
    LegacyPreferences { connections, theme }
}

fn parse_environment(value: &str) -> Environment {
    match value {
        "Production" => Environment::Production,
        "Development" => Environment::Development,
        "Local" => Environment::Local,
        _ => Environment::Test,
    }
}

fn parse_theme(value: &str) -> Option<AppThemeMode> {
    match value.trim() {
        "Light" => Some(AppThemeMode::Light),
        "Dark" => Some(AppThemeMode::Dark),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn node(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
    }

    #[test]
    fn builds_connections_with_fresh_ids_and_sorting() {
        let nodes = LegacyNodes {
            connections: vec![
                node(&[("name", "Prod"), ("restUrl", "https://prod.example.kz/engine-rest/"), ("environment", "Production")]),
                node(&[("name", ""), ("restUrl", "https://skip.example.kz/engine-rest")]),
                node(&[("name", "no rest"), ("restUrl", "")]),
                node(&[("name", "dev"), ("restUrl", "http://localhost:8080/engine-rest"), ("environment", "Weird")]),
                node(&[("name", "Dev"), ("restUrl", "http://localhost:9090/engine-rest"), ("environment", "Local")]),
            ],
            theme: Some("Purple".into()),
        };
        let preferences = build_legacy_preferences(nodes);
        let names: Vec<&str> = preferences.connections.iter().map(|connection| connection.name.as_str()).collect();
        assert_eq!(names, ["dev", "Dev", "Prod"]);
        let ids: Vec<&str> = preferences.connections.iter().map(|connection| connection.id.as_str()).collect();
        assert!(ids.contains(&"dev") && ids.contains(&"dev-2") && ids.contains(&"prod"));
        assert_eq!(preferences.connections[0].environment, Environment::Test);
        assert_eq!(preferences.connections[1].environment, Environment::Local);
        assert_eq!(preferences.connections[2].rest_url, "https://prod.example.kz/engine-rest");
        assert_eq!(preferences.connections[2].cockpit_url, "https://prod.example.kz/camunda/app/cockpit");
        assert_eq!(preferences.theme, None);
    }
}
