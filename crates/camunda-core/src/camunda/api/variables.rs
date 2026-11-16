use serde_json::{Value, json};

use super::json::{json_object, string};
use super::{ApiResult, CamundaApi};
use crate::camunda::http::encode;
use crate::models::*;

impl CamundaApi {
    // Недопустимое значение переменной отклоняется до обращения к сети.
    pub async fn update_process_variable(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
        update: &ProcessVariableUpdate,
    ) -> Result<(), String> {
        let url = format!("{}/process-instance/{}/variables/{}", connection.rest_url, encode(process_instance_id), encode(&update.name));
        let value = variable_json_value(update)?;
        let value_info = match update.value_info.as_deref() {
            None => json!({}),
            Some(raw) => match serde_json::from_str::<Value>(raw) {
                Ok(parsed @ Value::Object(_)) => parsed,
                Ok(_) => json!({}),
                Err(_) => return Err(rust_i18n::t!("core.variables.invalid_value_info").to_string()),
            },
        };
        let body = json!({ "value": value, "type": update.value_type, "valueInfo": value_info });
        self.mutate(&url, reqwest::Method::PUT, Some(body)).await
    }
}

fn variable_json_value(update: &ProcessVariableUpdate) -> Result<Value, String> {
    let value = update.value.trim();
    match update.value_type.to_lowercase().as_str() {
        "boolean" => match value.to_lowercase().as_str() {
            "true" => Ok(Value::from(true)),
            "false" => Ok(Value::from(false)),
            _ => Err(rust_i18n::t!("core.variables.invalid_boolean").to_string()),
        },
        "short" => {
            value.parse::<i16>().map(Value::from).map_err(|_| rust_i18n::t!("core.variables.invalid_integer", kind = "Short").to_string())
        }
        "integer" => {
            value.parse::<i32>().map(Value::from).map_err(|_| rust_i18n::t!("core.variables.invalid_integer", kind = "Integer").to_string())
        }
        "long" => {
            value.parse::<i64>().map(Value::from).map_err(|_| rust_i18n::t!("core.variables.invalid_integer", kind = "Long").to_string())
        }
        "double" => value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(Value::from)
            .ok_or_else(|| rust_i18n::t!("core.variables.invalid_double").to_string()),
        "null" => Ok(Value::Null),
        "json" => serde_json::from_str::<Value>(&update.value)
            .map(|_| Value::from(update.value.as_str()))
            .map_err(|_| rust_i18n::t!("core.variables.invalid_json").to_string()),
        _ => Ok(Value::from(update.value.as_str())),
    }
}

pub(super) fn parse_variables(body: &str) -> ApiResult<Vec<ProcessVariable>> {
    let mut variables: Vec<ProcessVariable> = json_object(body)?
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, value)| ProcessVariable {
            name: name.clone(),
            value_type: string(value, "type").unwrap_or_else(|| rust_i18n::t!("core.variables.unknown_type").to_string()),
            value: match value.get("value") {
                None | Some(Value::Null) => "null".to_string(),
                Some(Value::String(text)) => text.clone(),
                Some(Value::Number(number)) => number.to_string(),
                Some(Value::Bool(flag)) => flag.to_string(),
                Some(other) => other.to_string(),
            },
            value_info: value.get("valueInfo").map(Value::to_string).filter(|info| info != "{}"),
        })
        .collect();
    variables.sort_by_cached_key(|variable| variable.name.to_lowercase());
    Ok(variables)
}

#[cfg(test)]
mod tests {
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn variable_update_preserves_type_and_value_info() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;

        api()
            .update_process_variable(
                &connection(&server),
                "instance-1",
                &ProcessVariableUpdate {
                    name: "approved by support".to_string(),
                    value_type: "Boolean".to_string(),
                    value: "true".to_string(),
                    value_info: Some("{}".to_string()),
                },
            )
            .await
            .expect("обновление переменной");

        let requests = server.received_requests().await.unwrap_or_default();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "PUT");
        assert_eq!(requests[0].url.path(), "/engine-rest/process-instance/instance-1/variables/approved+by+support");
        let payload = body_json(&requests[0]);
        assert_eq!(payload["type"], "Boolean");
        assert_eq!(payload["value"], true);
        assert!(payload["value"].is_boolean());
        assert_eq!(payload["valueInfo"], json!({}));
    }

    #[tokio::test]
    async fn invalid_json_variable_is_rejected_before_request() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;
        let connection = connection(&server);
        let update = |value_type: &str, value: &str| ProcessVariableUpdate {
            name: "payload".to_string(),
            value_type: value_type.to_string(),
            value: value.to_string(),
            value_info: Some("{}".to_string()),
        };

        let json_failure = api().update_process_variable(&connection, "instance-1", &update("Json", "{broken")).await.expect_err("JSON");
        let long_failure = api().update_process_variable(&connection, "instance-1", &update("Long", "12.5")).await.expect_err("Long");
        let boolean_failure =
            api().update_process_variable(&connection, "instance-1", &update("Boolean", "да")).await.expect_err("Boolean");

        assert!(json_failure.contains("valid JSON"), "{json_failure}");
        assert_eq!(long_failure, "The value must be a Long integer.");
        assert_eq!(boolean_failure, "Boolean accepts only true or false.");
        assert!(server.received_requests().await.unwrap_or_default().is_empty());
    }
}
