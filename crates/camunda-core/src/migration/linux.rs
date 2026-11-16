use std::collections::HashMap;

#[cfg(target_os = "linux")]
use super::LegacyNodes;

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

#[cfg(target_os = "linux")]
pub(super) fn read_legacy_nodes() -> Option<LegacyNodes> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
