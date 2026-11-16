mod api_error;
mod client;
mod network_error;
mod user_message;

pub use api_error::ApiError;
pub use client::{CONNECT_TIMEOUT, REQUEST_TIMEOUT, camunda_client, send};
#[cfg(test)]
pub use network_error::describe_network_error;
pub use network_error::{NetworkErrorKind, classify_network_error, describe_network_kind};
pub use user_message::user_message;

// Кодирует значение для URL так же, как java.net.URLEncoder: пробел становится плюсом.
pub fn encode(value: &str) -> String {
    urlencoding::encode(value).replace("%20", "+")
}
