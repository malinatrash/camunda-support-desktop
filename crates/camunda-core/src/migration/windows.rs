#[cfg(windows)]
use super::LegacyNodes;

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

#[cfg(windows)]
const CONNECTION_KEYS: [&str; 5] = ["name", "restUrl", "cockpitUrl", "environment", "engineVersion"];

#[cfg(windows)]
pub(super) fn read_legacy_nodes() -> Option<LegacyNodes> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let root = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\JavaSoft\Prefs\com\malinatrash\camunda-support").ok()?;
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
    nodes.theme = root.open_subkey("settings").ok().and_then(|settings| settings.get_value::<String, _>(encode_windows_name("theme")).ok());
    Some(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
