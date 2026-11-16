mod connection_link;
mod parse;
mod raw_link;

pub use connection_link::{matches_origin, normalized_web_origin, process_instance_deep_link};
pub use parse::parse_deep_link;

use crate::models::ProcessInstanceDeepLink;

pub const APP_DEEP_LINK_SCHEME: &str = "camunda-support";

pub fn is_valid_process_instance_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= 256
        && !value.chars().any(|char| char.is_whitespace() || char.is_control() || matches!(char, '/' | '?' | '#'))
}

impl ProcessInstanceDeepLink {
    pub fn new(origin: impl Into<String>, process_instance_id: impl Into<String>) -> Self {
        Self { origin: origin.into(), process_instance_id: process_instance_id.into() }
    }
}
