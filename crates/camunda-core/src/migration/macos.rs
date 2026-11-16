use super::LegacyNodes;

// Разбирает вывод `plutil -convert json`: узлы оканчиваются на `/`, значения лежат строками.
pub fn parse_macos_plist_json(json: &str) -> Option<LegacyNodes> {
    let root: serde_json::Value = serde_json::from_str(json).ok()?;
    let node = root.get("/com/malinatrash/camunda-support/")?.as_object()?;
    let mut nodes = LegacyNodes::default();
    if let Some(connections) = node.get("connections/").and_then(|value| value.as_object()) {
        for (child, values) in connections {
            if !child.ends_with('/') {
                continue;
            }
            let Some(values) = values.as_object() else { continue };
            nodes.connections.push(values.iter().filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string()))).collect());
        }
    }
    nodes.theme = node.get("settings/").and_then(|settings| settings.get("theme")).and_then(|theme| theme.as_str()).map(str::to_string);
    Some(nodes)
}

#[cfg(target_os = "macos")]
pub(super) fn read_legacy_nodes() -> Option<LegacyNodes> {
    let path = std::path::PathBuf::from(std::env::var_os("HOME")?).join("Library/Preferences/com.malinatrash.camunda-support.plist");
    if !path.is_file() {
        return None;
    }
    let output = std::process::Command::new("plutil").args(["-convert", "json", "-o", "-"]).arg(&path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    parse_macos_plist_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::build_legacy_preferences;
    use crate::models::{AppThemeMode, Environment};

    #[test]
    fn parses_macos_plist_json() {
        let json = r#"{"\/com\/malinatrash\/camunda-support\/":{"tests\/":{},"connections\/":{
            "dev\/":{"engineVersion":"7.17.0","environment":"Development","cockpitUrl":"https:\/\/dev.example.kz\/camunda\/app\/cockpit","name":"dev","restUrl":"https:\/\/dev.example.kz\/engine-rest"},
            "залоги-test\/":{"environment":"Test","cockpitUrl":"","name":"Залоги TEST","restUrl":"https:\/\/test.example.kz\/engine-rest"}
        },"settings\/":{"theme":"Dark"}}}"#;
        let nodes = parse_macos_plist_json(json).expect("plist разобран");
        assert_eq!(nodes.theme.as_deref(), Some("Dark"));
        assert_eq!(nodes.connections.len(), 2);
        assert!(nodes.connections.iter().any(|values| values.get("name").map(String::as_str) == Some("dev")));

        let preferences = build_legacy_preferences(nodes);
        assert_eq!(preferences.theme, Some(AppThemeMode::Dark));
        assert_eq!(preferences.connections.len(), 2);
        let dev = &preferences.connections[0];
        assert_eq!(dev.id, "dev");
        assert_eq!(dev.name, "dev");
        assert_eq!(dev.environment, Environment::Development);
        assert_eq!(dev.engine_version.as_deref(), Some("7.17.0"));
        let test = &preferences.connections[1];
        assert_eq!(test.id, "залоги-test");
        assert_eq!(test.cockpit_url, "https://test.example.kz/camunda/app/cockpit");
        assert_eq!(test.engine_version, None);
        assert_eq!(parse_macos_plist_json("{}"), None);
        assert_eq!(parse_macos_plist_json("not json"), None);
    }
}
