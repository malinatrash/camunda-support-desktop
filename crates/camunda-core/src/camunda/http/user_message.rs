use super::{ApiError, describe_network_kind};

// Собирает сообщение об ошибке операции в формате "{message}\nЗапрос: {url}", как в старом приложении.
pub fn user_message(error: &ApiError, url: &str) -> String {
    let (message, request_url) = match error {
        ApiError::Http { status, server_message, request_url } => {
            (http_status_message(*status, server_message.as_deref()), request_url.as_str())
        }
        ApiError::Network(kind) => (describe_network_kind(kind), url),
        ApiError::Invalid(message) => {
            let message =
                if message.trim().is_empty() { "Некорректное значение запроса." } else { message.as_str() };
            (message.to_string(), url)
        }
        ApiError::Other(message) => (format!("Не удалось выполнить операцию: {message}"), url),
    };
    format!("{}\nЗапрос: {request_url}", message.trim())
}

fn http_status_message(status: u16, server_message: Option<&str>) -> String {
    let server_message = server_message.unwrap_or_default();
    match status {
        400 => format!("Camunda отклонила запрос (HTTP 400). {server_message}"),
        401 => "Требуется авторизация (HTTP 401).".to_string(),
        403 => "Доступ запрещён (HTTP 403). У подключения недостаточно прав.".to_string(),
        404 => "REST endpoint Camunda не найден (HTTP 404).".to_string(),
        429 => "Слишком много запросов (HTTP 429). Подождите и повторите попытку.".to_string(),
        500..=599 => format!("Camunda или gateway вернули HTTP {status}. {server_message}"),
        _ => format!("Camunda вернула HTTP {status}. {server_message}"),
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
            "Camunda отклонила запрос (HTTP 400). bad variable\nЗапрос: http://camunda/engine-rest/job"
        );
        let forbidden = ApiError::Http { status: 403, server_message: None, request_url: "u".to_string() };
        assert_eq!(user_message(&forbidden, "u"), "Доступ запрещён (HTTP 403). У подключения недостаточно прав.\nЗапрос: u");
        let server = ApiError::Http { status: 502, server_message: None, request_url: "u".to_string() };
        assert_eq!(user_message(&server, "u"), "Camunda или gateway вернули HTTP 502.\nЗапрос: u");
    }

    #[test]
    fn formats_invalid_and_other_errors() {
        assert_eq!(user_message(&ApiError::Invalid(String::new()), "u"), "Некорректное значение запроса.\nЗапрос: u");
        assert_eq!(user_message(&ApiError::Other("boom".to_string()), "u"), "Не удалось выполнить операцию: boom\nЗапрос: u");
        assert_eq!(encode("approved by support"), "approved+by+support");
        assert_eq!(encode("alpha:2:abc"), "alpha%3A2%3Aabc");
    }
}
