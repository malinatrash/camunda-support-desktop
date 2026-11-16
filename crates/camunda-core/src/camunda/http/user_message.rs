use super::{ApiError, describe_network_kind};

// Собирает сообщение об ошибке операции в формате "{message}\nЗапрос: {url}", как в старом приложении.
pub fn user_message(error: &ApiError, url: &str) -> String {
    let (message, request_url) = match error {
        ApiError::Http { status, server_message, request_url } => {
            (http_status_message(*status, server_message.as_deref()), request_url.as_str())
        }
        ApiError::Network(kind) => (describe_network_kind(kind), url),
        ApiError::Invalid(message) => {
            let message = if message.trim().is_empty() { rust_i18n::t!("core.http.invalid_value").to_string() } else { message.clone() };
            (message, url)
        }
        ApiError::Other(message) => (rust_i18n::t!("core.http.operation_failed", message = message).to_string(), url),
    };
    format!("{}\n{}", message.trim(), rust_i18n::t!("core.http.request_url", url = request_url))
}

fn http_status_message(status: u16, server_message: Option<&str>) -> String {
    let server_message = server_message.unwrap_or_default();
    match status {
        400 => rust_i18n::t!("core.http.status_400", server_message = server_message).to_string(),
        401 => rust_i18n::t!("core.http.status_401").to_string(),
        403 => rust_i18n::t!("core.http.status_403").to_string(),
        404 => rust_i18n::t!("core.http.status_404").to_string(),
        429 => rust_i18n::t!("core.http.too_many_requests").to_string(),
        500..=599 => rust_i18n::t!("core.http.status_5xx", status = status, server_message = server_message).to_string(),
        _ => rust_i18n::t!("core.http.status_other", status = status, server_message = server_message).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::http::encode;

    #[test]
    fn formats_http_errors_with_request_url() {
        let error = ApiError::Http {
            status: 400,
            server_message: Some("bad variable".to_string()),
            request_url: "http://camunda/engine-rest/job".to_string(),
        };
        assert_eq!(
            user_message(&error, "ignored"),
            "Camunda rejected the request (HTTP 400). bad variable\nRequest: http://camunda/engine-rest/job"
        );
        let forbidden = ApiError::Http { status: 403, server_message: None, request_url: "u".to_string() };
        assert_eq!(user_message(&forbidden, "u"), "Access denied (HTTP 403). The connection lacks permissions.\nRequest: u");
        let server = ApiError::Http { status: 502, server_message: None, request_url: "u".to_string() };
        assert_eq!(user_message(&server, "u"), "Camunda or the gateway returned HTTP 502.\nRequest: u");
    }

    #[test]
    fn formats_invalid_and_other_errors() {
        assert_eq!(user_message(&ApiError::Invalid(String::new()), "u"), "Invalid request value.\nRequest: u");
        assert_eq!(user_message(&ApiError::Other("boom".to_string()), "u"), "Operation failed: boom\nRequest: u");
        assert_eq!(encode("approved by support"), "approved+by+support");
        assert_eq!(encode("alpha:2:abc"), "alpha%3A2%3Aabc");
    }
}
