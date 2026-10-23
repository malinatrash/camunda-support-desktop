use std::collections::HashMap;

use crate::connection::{connection_id, derive_cockpit_url};
use crate::models::{AppThemeMode, CamundaConnection, Environment};

// Данные старой Kotlin-версии, прочитанные из java.util.prefs этого пользователя.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LegacyPreferences {
    pub connections: Vec<CamundaConnection>,
    pub theme: Option<AppThemeMode>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LegacyNodes {
    pub connections: Vec<HashMap<String, String>>,
    pub theme: Option<String>,
}

#[cfg(windows)]
const CONNECTION_KEYS: [&str; 5] = ["name", "restUrl", "cockpitUrl", "environment", "engineVersion"];

// Читает подключения и тему старой версии (java.util.prefs) на текущей ОС; при любой проблеме — пусто.
pub fn read_legacy_preferences() -> LegacyPreferences {
    build_legacy_preferences(read_legacy_nodes().unwrap_or_default())
}

#[cfg(target_os = "macos")]
fn read_legacy_nodes() -> Option<LegacyNodes> {
    let path = std::path::PathBuf::from(std::env::var_os("HOME")?)
        .join("Library/Preferences/com.malinatrash.camunda-support.plist");
    if !path.is_file() {
        return None;
    }
    let output = std::process::Command::new("plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(&path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_macos_plist_json(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "linux")]
fn read_legacy_nodes() -> Option<LegacyNodes> {
    let root = std::path::PathBuf::from(std::env::var_os("HOME")?).join(".java/.userPrefs/com/malinatrash/camunda-support");
    let mut nodes = LegacyNodes::default();
    for entry in std::fs::read_dir(root.join("connections")).ok()?.flatten() {
        let Ok(xml) = std::fs::read_to_string(entry.path().join("prefs.xml")) else { continue };
        if let Some(values) = parse_linux_prefs_xml(&xml) {
            nodes.connections.push(values);
        }
    }
    nodes.theme = std::fs::read_to_string(root.join("settings/prefs.xml"))
        .ok()
        .and_then(|xml| parse_linux_prefs_xml(&xml))
        .and_then(|mut values| values.remove("theme"));
    Some(nodes)
}

#[cfg(windows)]
fn read_legacy_nodes() -> Option<LegacyNodes> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let root = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\JavaSoft\Prefs\com\malinatrash\camunda-support")
        .ok()?;
    let mut nodes = LegacyNodes::default();
    if let Ok(connections) = root.open_subkey("connections") {
        for name in connections.enum_keys().flatten() {
            let Ok(node) = connections.open_subkey(&name) else { continue };
            let values = CONNECTION_KEYS
                .iter()
                .filter_map(|key| {
                    let value: String = node.get_value(encode_windows_name(key)).ok()?;
                    Some((key.to_string(), value))
                })
                .collect();
            nodes.connections.push(values);
        }
    }
    nodes.theme = root
        .open_subkey("settings")
        .ok()
        .and_then(|settings| settings.get_value::<String, _>(encode_windows_name("theme")).ok());
    Some(nodes)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn read_legacy_nodes() -> Option<LegacyNodes> {
    None
}

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
            nodes.connections.push(
                values
                    .iter()
                    .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
                    .collect(),
            );
        }
    }
    nodes.theme = node
        .get("settings/")
        .and_then(|settings| settings.get("theme"))
        .and_then(|theme| theme.as_str())
        .map(str::to_string);
    Some(nodes)
}

// Разбирает prefs.xml java.util.prefs: `<map><entry key="..." value="..."/></map>`.
#[allow(dead_code)]
pub fn parse_linux_prefs_xml(xml: &str) -> Option<HashMap<String, String>> {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let document = roxmltree::Document::parse_with_options(xml, options).ok()?;
    Some(
        document
            .descendants()
            .filter(|node| node.has_tag_name("entry"))
            .filter_map(|node| Some((node.attribute("key")?.to_string(), node.attribute("value")?.to_string())))
            .collect(),
    )
}

// Кодирует имя как java.util.prefs на Windows: `A` → `/A`, `/` → `\`, `\` → `//`, не-ASCII → `/uXXXX`.
#[allow(dead_code)]
pub fn encode_windows_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for char in name.chars() {
        match char {
            'A'..='Z' => {
                result.push('/');
                result.push(char);
            }
            '/' => result.push('\\'),
            '\\' => result.push_str("//"),
            '\u{20}'..='\u{7E}' => result.push(char),
            other => {
                for unit in other.encode_utf16(&mut [0; 2]) {
                    result.push_str(&format!("/u{unit:04x}"));
                }
            }
        }
    }
    result
}

// Обратное преобразование к encode_windows_name.
#[allow(dead_code)]
pub fn decode_windows_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut units: Vec<u16> = Vec::with_capacity(chars.len());
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '\\' => units.push('/' as u16),
            '/' => match chars.get(index + 1) {
                Some('/') => {
                    units.push('\\' as u16);
                    index += 1;
                }
                Some('u') => {
                    let hex: String = chars.iter().skip(index + 2).take(4).collect();
                    match (hex.len() == 4).then(|| u16::from_str_radix(&hex, 16).ok()).flatten() {
                        Some(unit) => {
                            units.push(unit);
                            index += 5;
                        }
                        None => units.push('/' as u16),
                    }
                }
                Some(upper @ 'A'..='Z') => {
                    units.push(*upper as u16);
                    index += 1;
                }
                _ => units.push('/' as u16),
            },
            other => units.extend(other.encode_utf16(&mut [0; 2]).iter().copied()),
        }
        index += 1;
    }
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
    }

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

    #[test]
    fn parses_linux_prefs_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<!DOCTYPE map SYSTEM "http://java.sun.com/dtd/preferences.dtd">
<map MAP_XML_VERSION="1.0">
  <entry key="name" value="Залоги TEST"/>
  <entry key="restUrl" value="https://test.example.kz/engine-rest"/>
  <entry key="environment" value="Test"/>
</map>"#;
        let values = parse_linux_prefs_xml(xml).expect("xml разобран");
        assert_eq!(values.get("name").map(String::as_str), Some("Залоги TEST"));
        assert_eq!(values.get("restUrl").map(String::as_str), Some("https://test.example.kz/engine-rest"));
        assert_eq!(values.len(), 3);
        assert_eq!(parse_linux_prefs_xml("<map"), None);
    }

    #[test]
    fn encodes_and_decodes_windows_names_like_java() {
        assert_eq!(encode_windows_name("restUrl"), "rest/Url");
        assert_eq!(encode_windows_name("cockpitUrl"), "cockpit/Url");
        assert_eq!(encode_windows_name("engineVersion"), "engine/Version");
        assert_eq!(encode_windows_name("a/b\\c"), "a\\b//c");
        assert_eq!(encode_windows_name("залог"), "/u0437/u0430/u043b/u043e/u0433");
        for name in ["restUrl", "a/b\\c", "залог-Test", "camunda-support", "Camunda Support"] {
            assert_eq!(decode_windows_name(&encode_windows_name(name)), name);
        }
        assert_eq!(decode_windows_name("/U0437"), "U0437");
        assert_eq!(decode_windows_name("/uzz"), "/uzz");
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
