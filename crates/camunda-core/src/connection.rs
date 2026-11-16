use crate::models::{CamundaConnection, ConnectionDraft, ConnectionValidation};

// Проверяет черновик подключения: название, REST URL с /engine-rest и необязательный Cockpit URL.
pub fn validate_connection_draft(draft: &ConnectionDraft) -> ConnectionValidation {
    fn generic_url_error(value: &str, required: bool) -> Option<String> {
        if value.trim().is_empty() {
            return required.then(|| rust_i18n::t!("core.connection.url_required").to_string());
        }
        let normalized = value.trim();
        if !normalized.starts_with("https://") && !normalized.starts_with("http://") {
            return Some(rust_i18n::t!("core.connection.url_scheme").to_string());
        }
        if normalized.chars().any(char::is_whitespace) {
            return Some(rust_i18n::t!("core.connection.url_whitespace").to_string());
        }
        if normalized.contains('?') || normalized.contains('#') {
            return Some(rust_i18n::t!("core.connection.url_query").to_string());
        }
        let authority_and_path = normalized.split_once("://").map(|(_, rest)| rest).unwrap_or("");
        if authority_and_path.split('/').next().unwrap_or("").trim().is_empty() {
            return Some(rust_i18n::t!("core.connection.url_host").to_string());
        }
        None
    }

    fn rest_url_error(value: &str) -> Option<String> {
        if let Some(error) = generic_url_error(value, true) {
            return Some(error);
        }
        let normalized = value.trim().trim_end_matches('/');
        if !normalized.ends_with("/engine-rest") {
            return Some(rust_i18n::t!("core.connection.rest_suffix").to_string());
        }
        None
    }

    ConnectionValidation {
        name_error: draft.name.trim().is_empty().then(|| rust_i18n::t!("core.connection.name_required").to_string()),
        rest_url_error: rest_url_error(&draft.rest_url),
        cockpit_url_error: generic_url_error(&draft.cockpit_url, false),
    }
}

// Выводит адрес Cockpit из домена REST URL: https://host/camunda/app/cockpit.
pub fn derive_cockpit_url(rest_url: &str) -> String {
    let normalized = rest_url.trim();
    let Some(scheme_separator) = normalized.find("://") else { return String::new() };
    if scheme_separator == 0 {
        return String::new();
    }
    let authority_start = scheme_separator + 3;
    let authority_end = normalized[authority_start..].find('/').map(|index| authority_start + index).unwrap_or(normalized.len());
    if authority_end <= authority_start {
        return String::new();
    }
    format!("{}/camunda/app/cockpit", &normalized[..authority_end])
}

pub fn draft_to_connection(draft: &ConnectionDraft, id: String, engine_version: Option<String>) -> CamundaConnection {
    let rest_url = draft.rest_url.trim().trim_end_matches('/').to_string();
    let cockpit_url = draft.cockpit_url.trim().trim_end_matches('/').to_string();
    CamundaConnection {
        id,
        name: draft.name.trim().to_string(),
        cockpit_url: if cockpit_url.is_empty() { derive_cockpit_url(&rest_url) } else { cockpit_url },
        rest_url,
        environment: draft.environment,
        engine_version,
    }
}

// Делает идентификатор подключения из названия: латиница, цифры и дефисы, уникальный среди existing.
pub fn connection_id(name: &str, existing: &[CamundaConnection]) -> String {
    let base: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|char| if char.is_alphanumeric() { char } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let base = if base.is_empty() { "connection".to_string() } else { base };
    let mut id = base.clone();
    let mut suffix = 2;
    while existing.iter().any(|connection| connection.id == id) {
        id = format!("{base}-{suffix}");
        suffix += 1;
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Environment;

    fn draft(name: &str, rest_url: &str, cockpit_url: &str) -> ConnectionDraft {
        ConnectionDraft { name: name.into(), rest_url: rest_url.into(), cockpit_url: cockpit_url.into(), environment: Environment::Test }
    }

    #[test]
    fn validates_required_fields() {
        let validation = validate_connection_draft(&draft("", "", ""));
        assert_eq!(validation.name_error.as_deref(), Some("Enter a name"));
        assert_eq!(validation.rest_url_error.as_deref(), Some("Enter a URL"));
        assert!(validation.cockpit_url_error.is_none());
    }

    #[test]
    fn has_russian_translations() {
        assert_eq!(rust_i18n::t!("core.connection.name_required", locale = "ru"), "Укажите название");
        assert_eq!(rust_i18n::t!("core.http.request_url", locale = "ru", url = "u"), "Запрос: u");
    }

    #[test]
    fn requires_engine_rest_suffix() {
        let validation = validate_connection_draft(&draft("Тест", "https://camunda.example.kz/rest", ""));
        assert_eq!(validation.rest_url_error.as_deref(), Some("Camunda REST URL must end with /engine-rest"));
        assert!(validate_connection_draft(&draft("Тест", "https://camunda.example.kz/engine-rest/", "")).is_valid());
    }

    #[test]
    fn rejects_query_and_whitespace() {
        assert_eq!(
            validate_connection_draft(&draft("a", "https://host/engine-rest?x=1", "")).rest_url_error.as_deref(),
            Some("URL must not contain query parameters or a fragment")
        );
        assert_eq!(
            validate_connection_draft(&draft("a", "ftp://host/engine-rest", "")).rest_url_error.as_deref(),
            Some("URL must start with https:// or http://")
        );
        assert_eq!(
            validate_connection_draft(&draft("a", "https://host/engine-rest", "https:// host")).cockpit_url_error.as_deref(),
            Some("URL must not contain spaces")
        );
    }

    #[test]
    fn derives_cockpit_url_from_rest_host() {
        assert_eq!(derive_cockpit_url("https://camunda.example.kz/engine-rest"), "https://camunda.example.kz/camunda/app/cockpit");
        assert_eq!(derive_cockpit_url("http://localhost:18080/engine-rest/"), "http://localhost:18080/camunda/app/cockpit");
        assert_eq!(derive_cockpit_url("nonsense"), "");
    }

    #[test]
    fn builds_unique_connection_ids() {
        let existing = vec![draft_to_connection(&draft("B2C Prod", "https://h/engine-rest", ""), "b2c-prod".into(), None)];
        assert_eq!(connection_id("B2C Prod", &existing), "b2c-prod-2");
        assert_eq!(connection_id("Залоги TEST", &[]), "залоги-test");
        assert_eq!(connection_id("!!!", &[]), "connection");
    }
}
