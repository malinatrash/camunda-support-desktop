use std::error::Error as StdError;

// Относит ошибку reqwest к одному из классов, как цепочка причин в Java (таймаут, DNS, TLS, отказ соединения, IO).
pub fn classify_network_error(error: &reqwest::Error) -> NetworkErrorKind {
    let mut causes = Vec::new();
    let mut io_message = None;
    let mut source = error.source();
    while let Some(cause) = source {
        if io_message.is_none()
            && let Some(io) = cause.downcast_ref::<std::io::Error>()
        {
            io_message = Some(io.to_string());
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
        NetworkErrorKind::ConnectionRefused => "Подключение отклонено или хост недоступен. Проверьте VPN, хост и порт.".to_string(),
        NetworkErrorKind::InvalidUrl => "Некорректный адрес запроса.".to_string(),
        NetworkErrorKind::Io(message) => format!("Сетевая ошибка: {message}"),
        NetworkErrorKind::Other(message) => format!("Не удалось выполнить операцию: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::http::camunda_client;

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
