use super::{APP_DEEP_LINK_SCHEME, is_valid_process_instance_id};
use crate::models::CamundaConnection;

// Строит ссылку camunda-support://process-instance/<id>?origin=<домен> для подключения.
pub fn process_instance_deep_link(connection: &CamundaConnection, process_instance_id: &str) -> Option<String> {
    if !is_valid_process_instance_id(process_instance_id) {
        return None;
    }
    let origin = normalized_web_origin(&connection.cockpit_url).or_else(|| normalized_web_origin(&connection.rest_url))?;
    Some(format!("{APP_DEEP_LINK_SCHEME}://process-instance/{}?origin={}", percent_encode(process_instance_id), percent_encode(&origin)))
}

// Проверяет, относится ли домен ссылки к Cockpit или REST адресу подключения.
pub fn matches_origin(connection: &CamundaConnection, origin: &str) -> bool {
    let Some(normalized_origin) = normalized_web_origin(origin) else {
        return false;
    };
    normalized_web_origin(&connection.cockpit_url).as_deref() == Some(normalized_origin.as_str())
        || normalized_web_origin(&connection.rest_url).as_deref() == Some(normalized_origin.as_str())
}

pub fn normalized_web_origin(value: &str) -> Option<String> {
    let normalized = value.trim();
    let scheme_separator = normalized.find("://").filter(|index| *index > 0)?;
    let scheme = normalized[..scheme_separator].to_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let authority_start = scheme_separator + 3;
    let authority_end =
        normalized[authority_start..].find(['/', '?', '#']).map(|index| authority_start + index).unwrap_or(normalized.len());
    if authority_end <= authority_start {
        return None;
    }
    let authority = normalized[authority_start..authority_end].to_lowercase();
    if authority.trim().is_empty() || authority.chars().any(char::is_whitespace) || authority.contains('@') {
        return None;
    }
    let default_port = if scheme == "http" { ":80" } else { ":443" };
    let authority = authority.strip_suffix(default_port).unwrap_or(&authority);
    if authority.trim().is_empty() {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

fn percent_encode(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Environment;

    fn connection(rest_url: &str, cockpit_url: &str) -> CamundaConnection {
        CamundaConnection {
            id: "test".into(),
            name: "Тест".into(),
            rest_url: rest_url.into(),
            cockpit_url: cockpit_url.into(),
            environment: Environment::Test,
            engine_version: None,
        }
    }

    #[test]
    fn creates_app_link_from_connection_origin() {
        let connection = connection("https://camunda.example.kz/engine-rest", "https://camunda.example.kz/camunda/app/cockpit");
        assert_eq!(
            process_instance_deep_link(&connection, "instance-42").as_deref(),
            Some("camunda-support://process-instance/instance-42?origin=https%3A%2F%2Fcamunda.example.kz")
        );
    }

    #[test]
    fn matches_exact_normalized_origin_only() {
        let connection = connection("https://CAMUNDA.example.kz:443/engine-rest", "");
        assert!(matches_origin(&connection, "https://camunda.example.kz"));
        assert!(!matches_origin(&connection, "https://other.example.kz"));
        assert!(!matches_origin(&connection, "http://camunda.example.kz"));
    }

    #[test]
    fn rejects_unsafe_process_instance_id() {
        let connection = connection("https://camunda.example.kz/engine-rest", "");
        assert_eq!(process_instance_deep_link(&connection, "instance/42"), None);
        assert_eq!(process_instance_deep_link(&connection, "instance 42"), None);
        assert!(!is_valid_process_instance_id(""));
        assert!(!is_valid_process_instance_id(&"a".repeat(257)));
    }

    #[test]
    fn normalizes_web_origin() {
        assert_eq!(normalized_web_origin(" HTTPS://Camunda.Example.kz:443/x?y#z ").as_deref(), Some("https://camunda.example.kz"));
        assert_eq!(normalized_web_origin("http://localhost:18080/engine-rest").as_deref(), Some("http://localhost:18080"));
        assert_eq!(normalized_web_origin("ftp://camunda.example.kz"), None);
        assert_eq!(normalized_web_origin("https://user@camunda.example.kz"), None);
        assert_eq!(normalized_web_origin("https://"), None);
        assert_eq!(normalized_web_origin("camunda.example.kz"), None);
    }
}
