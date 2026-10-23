use std::error::Error as StdError;
use std::time::Duration;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

// Общий HTTP-клиент для запросов к Camunda: таймауты и редиректы как в старом приложении.
pub fn camunda_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect("reqwest client")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkErrorKind {
    Timeout,
    UnknownHost,
    TlsHandshake,
    Tls,
    ConnectionRefused,
    InvalidUrl,
    Io(String),
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    Http { status: u16, server_message: Option<String>, request_url: String },
    Network(NetworkErrorKind),
    Invalid(String),
    Other(String),
}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        ApiError::Network(classify_network_error(&error))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        ApiError::Other(format!("Camunda вернула некорректный JSON: {error}"))
    }
}

// Относит ошибку reqwest к одному из классов, как цепочка причин в Java (таймаут, DNS, TLS, отказ соединения, IO).
pub fn classify_network_error(error: &reqwest::Error) -> NetworkErrorKind {
    let mut causes = Vec::new();
    let mut io_message = None;
    let mut source = error.source();
    while let Some(cause) = source {
        if io_message.is_none() {
            if let Some(io) = cause.downcast_ref::<std::io::Error>() {
                io_message = Some(io.to_string());
            }
        }
        causes.push(cause.to_string().to_lowercase());
        source = cause.source();
    }
    let text = causes.join(" | ");
    let contains = |needles: &[&str]| needles.iter().any(|needle| text.contains(needle));
    if error.is_timeout() || contains(&["timed out", "timeout"]) {
        return NetworkErrorKind::Timeout;
    }
    if contains(&["dns", "lookup", "resolve", "nodename nor servname", "name or service not known"]) {
        return NetworkErrorKind::UnknownHost;
    }
    if contains(&["handshake", "certificate"]) {
        return NetworkErrorKind::TlsHandshake;
    }
    if contains(&["tls", "ssl"]) {
        return NetworkErrorKind::Tls;
    }
    if error.is_connect() || contains(&["connection refused", "unreachable", "connection reset"]) {
        return NetworkErrorKind::ConnectionRefused;
    }
    if error.is_builder() {
        return NetworkErrorKind::InvalidUrl;
    }
    if let Some(message) = io_message {
        return NetworkErrorKind::Io(message);
    }
    NetworkErrorKind::Other(error.to_string())
}

// Переводит сетевую ошибку reqwest в русское сообщение для пользователя.
#[cfg(test)]
pub fn describe_network_error(error: &reqwest::Error) -> String {
    describe_network_kind(&classify_network_error(error))
}

pub fn describe_network_kind(kind: &NetworkErrorKind) -> String {
    match kind {
        NetworkErrorKind::Timeout => "Время ожидания истекло. Проверьте VPN и доступность Camunda.".to_string(),
        NetworkErrorKind::UnknownHost => "Хост Camunda не найден. Проверьте DNS, адрес и VPN.".to_string(),
        NetworkErrorKind::TlsHandshake | NetworkErrorKind::Tls => "Ошибка TLS. Проверьте HTTPS-сертификат.".to_string(),
        NetworkErrorKind::ConnectionRefused => {
            "Подключение отклонено или хост недоступен. Проверьте VPN, хост и порт.".to_string()
        }
        NetworkErrorKind::InvalidUrl => "Некорректный адрес запроса.".to_string(),
        NetworkErrorKind::Io(message) => format!("Сетевая ошибка: {message}"),
        NetworkErrorKind::Other(message) => format!("Не удалось выполнить операцию: {message}"),
    }
}

// Выполняет запрос к Camunda и возвращает тело ответа; любой статус вне 2xx становится ApiError::Http.
pub async fn send(client: &reqwest::Client, request: reqwest::RequestBuilder) -> Result<String, ApiError> {
    let request = request.header("Accept", "application/json").build()?;
    let request_url = request.url().to_string();
    let response = client.execute(request).await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ApiError::Http {
            status: status.as_u16(),
            server_message: extract_server_message(&body),
            request_url,
        });
    }
    Ok(body)
}

fn extract_server_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value.get("message")?.as_str().map(str::to_string)
}

// Собирает сообщение об ошибке операции в формате "{message}\nЗапрос: {url}", как в старом приложении.
pub fn user_message(error: &ApiError, url: &str) -> String {
    let (message, request_url) = match error {
        ApiError::Http { status, server_message, request_url } => {
            (http_status_message(*status, server_message.as_deref()), request_url.as_str())
        }
        ApiError::Network(kind) => (describe_network_kind(kind), url),
        ApiError::Invalid(message) => {
            let message = if message.trim().is_empty() { "Некорректное значение запроса." } else { message.as_str() };
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

// Кодирует значение для URL так же, как java.net.URLEncoder: пробел становится плюсом.
pub fn encode(value: &str) -> String {
    urlencoding::encode(value).replace("%20", "+")
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn refused_connection_is_classified() {
        let client = camunda_client();
        let error = client.get("http://127.0.0.1:1/engine-rest/version").send().await.expect_err("порт закрыт");
        assert_eq!(classify_network_error(&error), NetworkErrorKind::ConnectionRefused);
        assert_eq!(describe_network_error(&error), "Подключение отклонено или хост недоступен. Проверьте VPN, хост и порт.");
    }

    #[tokio::test]
    async fn invalid_url_is_classified() {
        let error = camunda_client().get("not a url").send().await.expect_err("адрес некорректен");
        assert_eq!(classify_network_error(&error), NetworkErrorKind::InvalidUrl);
    }
}
