use super::{NetworkErrorKind, classify_network_error};

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
        ApiError::Other(rust_i18n::t!("core.json.invalid", error = error).to_string())
    }
}
