use std::time::Duration;

use super::ApiError;

// Общий HTTP-клиент для запросов к Camunda: таймауты и редиректы как в старом приложении.
pub fn camunda_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect("reqwest client")
}

// Выполняет запрос к Camunda и возвращает тело ответа; любой статус вне 2xx становится ApiError::Http.
pub async fn send(client: &reqwest::Client, request: reqwest::RequestBuilder) -> Result<String, ApiError> {
    let request = request.header("Accept", "application/json").build()?;
    let request_url = request.url().to_string();
    let response = client.execute(request).await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(ApiError::Http { status: status.as_u16(), server_message: extract_server_message(&body), request_url });
    }
    Ok(body)
}

fn extract_server_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value.get("message")?.as_str().map(str::to_string)
}

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
