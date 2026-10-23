use crate::models::{CamundaConnection, DeepLinkEvent, ProcessInstanceDeepLink};

pub const APP_DEEP_LINK_SCHEME: &str = "camunda-support";

const INVALID_FORMAT: &str = "Ссылка имеет некорректный формат.";
const UNSUPPORTED_SCHEME: &str = "Приложение не поддерживает протокол этой ссылки.";
const NOT_PROCESS_INSTANCE: &str = "Ссылка не относится к заявке Camunda.";
const INVALID_PROCESS_INSTANCE_ID: &str = "В ссылке указан некорректный идентификатор заявки.";
const MISSING_ORIGIN: &str = "В ссылке отсутствует корректный домен Camunda.";

struct RawLink<'a> {
    scheme: Option<String>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

// Разбирает ссылку camunda-support:// или ссылку Cockpit в событие открытия заявки.
pub fn parse_deep_link(raw_value: &str) -> DeepLinkEvent {
    let Some(link) = split_link(raw_value.trim()) else {
        return invalid(INVALID_FORMAT);
    };
    match link.scheme.as_deref() {
        Some(APP_DEEP_LINK_SCHEME) => parse_application_link(&link),
        Some("http") | Some("https") => parse_cockpit_link(&link),
        _ => invalid(UNSUPPORTED_SCHEME),
    }
}

fn parse_application_link(link: &RawLink) -> DeepLinkEvent {
    if !link.authority.is_some_and(|authority| authority.eq_ignore_ascii_case("process-instance")) {
        return invalid(NOT_PROCESS_INSTANCE);
    }
    let process_instance_id = decode(link.path.trim_matches('/'));
    if !is_valid_process_instance_id(&process_instance_id) {
        return invalid(INVALID_PROCESS_INSTANCE_ID);
    }
    let Some(origin) = query_value(link.query, "origin").and_then(|origin| normalized_web_origin(&origin)) else {
        return invalid(MISSING_ORIGIN);
    };
    DeepLinkEvent::OpenProcessInstance { link: ProcessInstanceDeepLink::new(origin, process_instance_id) }
}

fn parse_cockpit_link(link: &RawLink) -> DeepLinkEvent {
    let scheme = link.scheme.as_deref().unwrap_or_default();
    let Some(origin) = normalized_web_origin(&format!("{scheme}://{}", link.authority.unwrap_or_default())) else {
        return invalid(MISSING_ORIGIN);
    };
    let fragment = link.fragment.unwrap_or_default();
    let route = fragment.split('?').next().unwrap_or_default().trim_matches('/');
    let segments: Vec<&str> = route.split('/').collect();
    if segments.len() < 2 || segments[0] != "process-instance" {
        return invalid(NOT_PROCESS_INSTANCE);
    }
    let process_instance_id = decode(segments[1]);
    if !is_valid_process_instance_id(&process_instance_id) {
        return invalid(INVALID_PROCESS_INSTANCE_ID);
    }
    DeepLinkEvent::OpenProcessInstance { link: ProcessInstanceDeepLink::new(origin, process_instance_id) }
}

// Делит ссылку на части как java.net.URI: пробелы и управляющие символы делают её некорректной.
fn split_link(value: &str) -> Option<RawLink<'_>> {
    if value.is_empty() || value.chars().any(|char| char.is_whitespace() || char.is_control()) {
        return None;
    }
    let (without_fragment, fragment) = match value.find('#') {
        Some(index) => (&value[..index], Some(&value[index + 1..])),
        None => (value, None),
    };
    let (without_query, query) = match without_fragment.find('?') {
        Some(index) => (&without_fragment[..index], Some(&without_fragment[index + 1..])),
        None => (without_fragment, None),
    };
    let scheme_end = without_query.find(':').filter(|index| !without_query[..*index].contains('/'));
    let (scheme, rest) = match scheme_end {
        Some(index) if is_valid_scheme(&without_query[..index]) => {
            (Some(without_query[..index].to_lowercase()), &without_query[index + 1..])
        }
        _ => (None, without_query),
    };
    let (authority, path) = match rest.strip_prefix("//") {
        Some(after) => {
            let end = after.find('/').unwrap_or(after.len());
            (Some(&after[..end]), &after[end..])
        }
        None => (None, rest),
    };
    Some(RawLink { scheme, authority, path, query, fragment })
}

fn is_valid_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|char| char.is_ascii_alphanumeric() || matches!(char, '+' | '-' | '.'))
}

fn query_value(raw_query: Option<&str>, expected_name: &str) -> Option<String> {
    raw_query?
        .split('&')
        .map(|parameter| match parameter.find('=') {
            Some(index) => (&parameter[..index], &parameter[index + 1..]),
            None => (parameter, ""),
        })
        .find(|(name, _)| decode(name) == expected_name)
        .map(|(_, value)| decode(value))
}

// Декодирует как java.net.URLDecoder: `+` → пробел, %XX → байт, битые последовательности остаются как есть.
fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex_pair = (bytes.get(index + 1).and_then(hex_digit), bytes.get(index + 2).and_then(hex_digit));
        match (bytes[index], hex_pair) {
            (b'+', _) => result.push(b' '),
            (b'%', (Some(high), Some(low))) => {
                result.push(high << 4 | low);
                index += 2;
            }
            (byte, _) => result.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn hex_digit(byte: &u8) -> Option<u8> {
    (*byte as char).to_digit(16).map(|digit| digit as u8)
}

fn invalid(message: &str) -> DeepLinkEvent {
    DeepLinkEvent::Invalid { message: message.to_string() }
}

// Строит ссылку camunda-support://process-instance/<id>?origin=<домен> для подключения.
pub fn process_instance_deep_link(connection: &CamundaConnection, process_instance_id: &str) -> Option<String> {
    if !is_valid_process_instance_id(process_instance_id) {
        return None;
    }
    let origin = normalized_web_origin(&connection.cockpit_url).or_else(|| normalized_web_origin(&connection.rest_url))?;
    Some(format!(
        "{APP_DEEP_LINK_SCHEME}://process-instance/{}?origin={}",
        percent_encode(process_instance_id),
        percent_encode(&origin)
    ))
}

// Проверяет, относится ли домен ссылки к Cockpit или REST адресу подключения.
#[cfg(test)]
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
    let authority_end = normalized[authority_start..]
        .find(['/', '?', '#'])
        .map(|index| authority_start + index)
        .unwrap_or(normalized.len());
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

pub fn is_valid_process_instance_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= 256
        && !value.chars().any(|char| char.is_whitespace() || char.is_control() || matches!(char, '/' | '?' | '#'))
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

impl ProcessInstanceDeepLink {
    pub fn new(origin: impl Into<String>, process_instance_id: impl Into<String>) -> Self {
        Self { origin: origin.into(), process_instance_id: process_instance_id.into() }
    }
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

    fn open_link(event: DeepLinkEvent) -> ProcessInstanceDeepLink {
        match event {
            DeepLinkEvent::OpenProcessInstance { link } => link,
            DeepLinkEvent::Invalid { message } => panic!("ожидалась ссылка на заявку, получено: {message}"),
        }
    }

    fn invalid_message(event: DeepLinkEvent) -> String {
        match event {
            DeepLinkEvent::Invalid { message } => message,
            other => panic!("ожидалась некорректная ссылка, получено: {other:?}"),
        }
    }

    #[test]
    fn creates_app_link_from_connection_origin() {
        let connection =
            connection("https://camunda.example.kz/engine-rest", "https://camunda.example.kz/camunda/app/cockpit");
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

    #[test]
    fn parses_cockpit_process_instance_url() {
        let link = open_link(parse_deep_link(
            "https://camunda-pledge.example.kz/camunda/app/cockpit/default/#/process-instance/1cf88e5f-8508-11f1-9c3c-b2aa3636eb4e/runtime?viewbox=%7B%7D",
        ));
        assert_eq!(link.origin, "https://camunda-pledge.example.kz");
        assert_eq!(link.process_instance_id, "1cf88e5f-8508-11f1-9c3c-b2aa3636eb4e");
    }

    #[test]
    fn parses_application_deep_link() {
        let link = open_link(parse_deep_link(
            "camunda-support://process-instance/instance-42?origin=https%3A%2F%2Fcamunda.example.kz",
        ));
        assert_eq!(link.origin, "https://camunda.example.kz");
        assert_eq!(link.process_instance_id, "instance-42");
    }

    #[test]
    fn rejects_application_link_without_camunda_origin() {
        assert_eq!(invalid_message(parse_deep_link("camunda-support://process-instance/instance-42")), MISSING_ORIGIN);
    }

    #[test]
    fn reports_specific_reasons_for_invalid_links() {
        assert_eq!(invalid_message(parse_deep_link("camunda-support://task/instance-42")), NOT_PROCESS_INSTANCE);
        assert_eq!(invalid_message(parse_deep_link("camunda-support://process-instance/")), INVALID_PROCESS_INSTANCE_ID);
        assert_eq!(invalid_message(parse_deep_link("ftp://camunda.example.kz/#/process-instance/1")), UNSUPPORTED_SCHEME);
        assert_eq!(invalid_message(parse_deep_link("just text")), INVALID_FORMAT);
        assert_eq!(invalid_message(parse_deep_link("")), INVALID_FORMAT);
        assert_eq!(invalid_message(parse_deep_link("https://camunda.example.kz/camunda/app/cockpit/default/#/process-definition/1")), NOT_PROCESS_INSTANCE);
        assert_eq!(invalid_message(parse_deep_link("https:///#/process-instance/1")), MISSING_ORIGIN);
    }

    #[test]
    fn decodes_percent_sequences_like_java() {
        assert_eq!(decode("a+b%20c%2F"), "a b c/");
        assert_eq!(decode("%D0%B7%D0%B0"), "за");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz"), "%zz");
    }
}
