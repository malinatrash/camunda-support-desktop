use serde_json::Value;

use super::ApiResult;
use crate::camunda::http::ApiError;

pub(super) fn json_array(body: &str) -> ApiResult<Vec<Value>> {
    match serde_json::from_str::<Value>(body)? {
        Value::Array(values) => Ok(values),
        _ => Err(ApiError::Other(rust_i18n::t!("core.json.not_array").to_string())),
    }
}

pub(super) fn json_object(body: &str) -> ApiResult<Value> {
    let value: Value = serde_json::from_str(body)?;
    if value.is_object() { Ok(value) } else { Err(ApiError::Other(rust_i18n::t!("core.json.not_object").to_string())) }
}

pub(super) fn as_array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

pub(super) fn string(value: &Value, name: &str) -> Option<String> {
    value.get(name).and_then(primitive_content)
}

pub(super) fn int(value: &Value, name: &str) -> i32 {
    nullable_int(value, name).unwrap_or(0)
}

pub(super) fn nullable_int(value: &Value, name: &str) -> Option<i32> {
    match value.get(name)? {
        Value::Number(number) => number.as_i64().and_then(|number| i32::try_from(number).ok()),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

pub(super) fn boolean(value: &Value, name: &str) -> bool {
    nullable_boolean(value, name).unwrap_or(false)
}

pub(super) fn nullable_boolean(value: &Value, name: &str) -> Option<bool> {
    match value.get(name)? {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

pub(super) fn string_array(value: &Value, name: &str) -> Vec<String> {
    as_array(value.get(name)).iter().filter_map(primitive_content).collect()
}

pub(super) fn parse_count(body: &str) -> ApiResult<i32> {
    Ok(int(&json_object(body)?, "count"))
}

fn primitive_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}
