use std::time::Duration;

use crate::camunda::http::{NetworkErrorKind, classify_network_error};
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
    let body = response.text().await.map_err(|error| failure(&network_message(&classify_network_error(&error)), &version_url))?;
    match status {
        200..=299 => Ok(ConnectionTestSuccess { engine_version: extract_version(&body) }),
        _ => Err(failure(&status_message(status), &version_url)),
    }
}

fn status_message(status: u16) -> String {
    match status {
        400 => rust_i18n::t!("core.tester.http_400").to_string(),
        401 => rust_i18n::t!("core.tester.http_401").to_string(),
        403 => rust_i18n::t!("core.tester.http_403").to_string(),
        404 => rust_i18n::t!("core.tester.http_404").to_string(),
        405 => rust_i18n::t!("core.tester.http_405").to_string(),
        408 => rust_i18n::t!("core.tester.http_408").to_string(),
        429 => rust_i18n::t!("core.http.too_many_requests").to_string(),
        500..=599 => rust_i18n::t!("core.tester.http_5xx", status = status).to_string(),
        300..=399 => rust_i18n::t!("core.tester.http_3xx", status = status).to_string(),
        _ => rust_i18n::t!("core.tester.http_other", status = status).to_string(),
    }
}

fn network_message(kind: &NetworkErrorKind) -> String {
    match kind {
        NetworkErrorKind::Timeout => rust_i18n::t!("core.tester.timeout").to_string(),
        NetworkErrorKind::UnknownHost => rust_i18n::t!("core.tester.unknown_host").to_string(),
        NetworkErrorKind::TlsHandshake => rust_i18n::t!("core.tester.tls_handshake").to_string(),
        NetworkErrorKind::Tls => rust_i18n::t!("core.tester.tls").to_string(),
        NetworkErrorKind::ConnectionRefused => rust_i18n::t!("core.tester.connection_refused").to_string(),
        NetworkErrorKind::InvalidUrl => rust_i18n::t!("core.tester.invalid_url").to_string(),
        NetworkErrorKind::Io(message) => {
            let message = if message.trim().is_empty() { rust_i18n::t!("core.tester.request_failed").to_string() } else { message.clone() };
            rust_i18n::t!("core.tester.io", message = message).to_string()
        }
        NetworkErrorKind::Other(message) => rust_i18n::t!("core.tester.other", message = message).to_string(),
    }
}

fn failure(message: &str, version_url: &str) -> String {
    format!("{message}\n{}", rust_i18n::t!("core.tester.checked_url", url = version_url))
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
        let message = test_connection(&camunda_client(), &format!("{}/engine-rest", server.uri())).await.expect_err("ожидалась ошибка");
        assert!(message.contains("Authorization required (HTTP 401)"), "{message}");
        assert!(message.contains("/engine-rest/version"), "{message}");
        assert!(message.contains("\nChecked URL: "), "{message}");
    }

    #[tokio::test]
    async fn missing_endpoint_returns_specific_error() {
        let server = start_server(404, "Not found").await;
        let message = test_connection(&camunda_client(), &format!("{}/engine-rest", server.uri())).await.expect_err("ожидалась ошибка");
        assert!(message.contains("Camunda endpoint not found (HTTP 404)"), "{message}");
    }

    #[tokio::test]
    async fn refused_connection_returns_network_error() {
        let message = test_connection(&camunda_client(), "http://127.0.0.1:1/engine-rest").await.expect_err("порт закрыт");
        assert!(message.starts_with("Connection refused or host unreachable. Check the host, port, VPN and firewall."), "{message}");
        assert!(message.ends_with("Checked URL: http://127.0.0.1:1/engine-rest/version"), "{message}");
    }
}
