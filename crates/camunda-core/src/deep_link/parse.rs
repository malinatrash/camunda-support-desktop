use super::raw_link::{RawLink, decode, query_value, split_link};
use super::{APP_DEEP_LINK_SCHEME, is_valid_process_instance_id, normalized_web_origin};
use crate::models::{DeepLinkEvent, ProcessInstanceDeepLink};

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

fn invalid(message: &str) -> DeepLinkEvent {
    DeepLinkEvent::Invalid { message: message.to_string() }
}

const INVALID_FORMAT: &str = "Ссылка имеет некорректный формат.";
const UNSUPPORTED_SCHEME: &str = "Приложение не поддерживает протокол этой ссылки.";
const NOT_PROCESS_INSTANCE: &str = "Ссылка не относится к заявке Camunda.";
const INVALID_PROCESS_INSTANCE_ID: &str = "В ссылке указан некорректный идентификатор заявки.";
const MISSING_ORIGIN: &str = "В ссылке отсутствует корректный домен Camunda.";

#[cfg(test)]
mod tests {
    use super::*;

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
    fn parses_cockpit_process_instance_url() {
        let link = open_link(parse_deep_link(
            "https://camunda-pledge.example.kz/camunda/app/cockpit/default/#/process-instance/1cf88e5f-8508-11f1-9c3c-b2aa3636eb4e/runtime?viewbox=%7B%7D",
        ));
        assert_eq!(link.origin, "https://camunda-pledge.example.kz");
        assert_eq!(link.process_instance_id, "1cf88e5f-8508-11f1-9c3c-b2aa3636eb4e");
    }

    #[test]
    fn parses_application_deep_link() {
        let link = open_link(parse_deep_link("camunda-support://process-instance/instance-42?origin=https%3A%2F%2Fcamunda.example.kz"));
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
        assert_eq!(
            invalid_message(parse_deep_link("https://camunda.example.kz/camunda/app/cockpit/default/#/process-definition/1")),
            NOT_PROCESS_INSTANCE
        );
        assert_eq!(invalid_message(parse_deep_link("https:///#/process-instance/1")), MISSING_ORIGIN);
    }
}
