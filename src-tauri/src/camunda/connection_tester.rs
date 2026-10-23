use std::time::Duration;

use crate::camunda::http::{classify_network_error, NetworkErrorKind};
use crate::models::ConnectionTestSuccess;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// Проверяет подключение запросом GET {restUrl}/version и возвращает версию движка.
pub async fn test_connection(client: &reqwest::Client, rest_url: &str) -> Result<ConnectionTestSuccess, String> {
    let version_url = format!("{}/version", rest_url.trim().trim_end_matches('/'));
    let response = client
        .get(&version_url)
        .timeout(REQUEST_TIMEOUT)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| failure(&network_message(&classify_network_error(&error)), &version_url))?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|error| failure(&network_message(&classify_network_error(&error)), &version_url))?;
    match status {
        200..=299 => Ok(ConnectionTestSuccess { engine_version: extract_version(&body) }),
        _ => Err(failure(&status_message(status), &version_url)),
    }
}

fn status_message(status: u16) -> String {
    match status {
        400 => "Camunda отклонила проверочный запрос (HTTP 400). Проверьте REST URL.".to_string(),
        401 => "Требуется авторизация (HTTP 401). Подключение не имеет доступа к Camunda REST.".to_string(),
        403 => "Доступ запрещён (HTTP 403). У пользователя или сети нет доступа к Camunda REST.".to_string(),
        404 => "Endpoint Camunda не найден (HTTP 404). Проверьте, что REST API доступен по /engine-rest.".to_string(),
        405 => "Сервер запрещает GET /version (HTTP 405). Проверьте proxy и настройки Camunda REST.".to_string(),
        408 => "Camunda не успела обработать запрос (HTTP 408).".to_string(),
        429 => "Слишком много запросов (HTTP 429). Подождите и повторите попытку.".to_string(),
        500..=599 => format!("Camunda или gateway вернули серверную ошибку HTTP {status}."),
        300..=399 => format!("Неожиданный redirect HTTP {status}. Проверьте REST URL и gateway авторизации."),
        _ => format!("Неожиданный HTTP-ответ: {status}."),
    }
}

fn network_message(kind: &NetworkErrorKind) -> String {
    match kind {
        NetworkErrorKind::Timeout => "Время ожидания подключения истекло. Проверьте URL, VPN и сеть.".to_string(),
        NetworkErrorKind::UnknownHost => "Хост не найден — ошибка DNS. Проверьте имя хоста и VPN.".to_string(),
        NetworkErrorKind::TlsHandshake => {
            "Не удалось установить TLS-соединение. Сертификат сервера может быть недействительным или недоверенным."
                .to_string()
        }
        NetworkErrorKind::Tls => "Ошибка защищённого соединения TLS. Проверьте сертификат и настройку HTTPS.".to_string(),
        NetworkErrorKind::ConnectionRefused => {
            "Подключение отклонено или хост недоступен. Проверьте хост, порт, VPN и firewall.".to_string()
        }
        NetworkErrorKind::InvalidUrl => "Указан некорректный REST URL.".to_string(),
        NetworkErrorKind::Io(message) => {
            let message = if message.trim().is_empty() { "запрос не выполнен" } else { message.as_str() };
            format!("Сетевая ошибка: {message}.")
        }
        NetworkErrorKind::Other(message) => format!("Не удалось проверить подключение: {message}."),
    }
}

fn failure(message: &str, version_url: &str) -> String {
    format!("{message}\nПроверялся адрес: {version_url}")
}

fn extract_version(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value.get("version")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::camunda::http::camunda_client;

    async fn start_server(status: u16, body: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/engine-rest/version"))
            .respond_with(ResponseTemplate::new(status).set_body_raw(body.to_string(), "application/json"))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn successful_ping_returns_engine_version() {
        let server = start_server(200, r#"{"version":"7.21.0"}"#).await;
        let result = test_connection(&camunda_client(), &format!("{}/engine-rest/", server.uri())).await;
        assert_eq!(result, Ok(ConnectionTestSuccess { engine_version: Some("7.21.0".to_string()) }));
    }

    #[tokio::test]
    async fn unauthorized_ping_returns_specific_error() {
        let server = start_server(401, "Unauthorized").await;
        let message = test_connection(&camunda_client(), &format!("{}/engine-rest", server.uri()))
            .await
            .expect_err("ожидалась ошибка");
        assert!(message.contains("Требуется авторизация (HTTP 401)"), "{message}");
        assert!(message.contains("/engine-rest/version"), "{message}");
        assert!(message.contains("\nПроверялся адрес: "), "{message}");
    }

    #[tokio::test]
    async fn missing_endpoint_returns_specific_error() {
        let server = start_server(404, "Not found").await;
        let message = test_connection(&camunda_client(), &format!("{}/engine-rest", server.uri()))
            .await
            .expect_err("ожидалась ошибка");
        assert!(message.contains("Endpoint Camunda не найден (HTTP 404)"), "{message}");
    }

    #[tokio::test]
    async fn refused_connection_returns_network_error() {
        let message = test_connection(&camunda_client(), "http://127.0.0.1:1/engine-rest").await.expect_err("порт закрыт");
        assert!(message.starts_with("Подключение отклонено или хост недоступен. Проверьте хост, порт, VPN и firewall."), "{message}");
        assert!(message.ends_with("Проверялся адрес: http://127.0.0.1:1/engine-rest/version"), "{message}");
    }
}
