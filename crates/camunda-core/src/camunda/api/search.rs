use std::collections::HashSet;

use futures_util::future::try_join_all;
use serde_json::{Value, json};

use super::instances::parse_instances;
use super::{ApiResult, CamundaApi, SEARCH_LIMIT};
use crate::camunda::http::{self, ApiError};
use crate::models::*;

impl CamundaApi {
    pub async fn search_process_instances(
        &self,
        connection: &CamundaConnection,
        process_definition_key: &str,
        variable_name: &str,
        variable_value: &str,
        value_type: VariableValueType,
    ) -> Result<Vec<ProcessInstanceSummary>, String> {
        let url = format!("{}/process-instance?firstResult=0&maxResults={SEARCH_LIMIT}", connection.rest_url);
        let result = async {
            let url = &url;
            let searches = search_json_values(variable_value, value_type)?.into_iter().map(|typed_value| {
                let body = json!({
                    "processDefinitionKey": process_definition_key,
                    "variables": [{ "name": variable_name.trim(), "operator": "eq", "value": typed_value }],
                    "sorting": [{ "sortBy": "instanceId", "sortOrder": "desc" }],
                });
                async move { parse_instances(&self.post(url, Some(body)).await?) }
            });
            let pages = try_join_all(searches).await?;
            let mut seen = HashSet::new();
            Ok(pages.into_iter().flatten().filter(|instance| seen.insert(instance.id.clone())).take(SEARCH_LIMIT).collect())
        }
        .await;
        result.map_err(|error| http::user_message(&error, &url))
    }
}

// Для «Авто» подбирает все правдоподобные типы значения: строку, число и логическое.
fn search_json_values(value: &str, value_type: VariableValueType) -> ApiResult<Vec<Value>> {
    let trimmed = value.trim();
    let as_long = trimmed.parse::<i64>().ok();
    let as_double = trimmed.parse::<f64>().ok().filter(|number| number.is_finite());
    let as_boolean = match trimmed.to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };
    match value_type {
        VariableValueType::Auto => {
            let mut values = vec![Value::from(value)];
            if let Some(number) = as_long {
                values.push(Value::from(number));
            } else if let Some(number) = as_double {
                values.push(Value::from(number));
            }
            if let Some(flag) = as_boolean {
                values.push(Value::from(flag));
            }
            let mut seen = HashSet::new();
            Ok(values.into_iter().filter(|value| seen.insert(value.to_string())).collect())
        }
        VariableValueType::String => Ok(vec![Value::from(value)]),
        VariableValueType::Number => as_long
            .map(Value::from)
            .or_else(|| as_double.map(Value::from))
            .map(|number| vec![number])
            .ok_or_else(|| ApiError::Invalid(rust_i18n::t!("core.search.invalid_number").to_string())),
        VariableValueType::Boolean => as_boolean
            .map(|flag| vec![Value::from(flag)])
            .ok_or_else(|| ApiError::Invalid(rust_i18n::t!("core.search.invalid_boolean").to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn variable_search_sends_typed_post_body_and_parses_instances() {
        let server = MockServer::start().await;
        mount(
            &server,
            "POST",
            "/engine-rest/process-instance",
            200,
            r#"[{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","businessKey":"BR-42","ended":false,"suspended":false,"tenantId":"b2c"}]"#,
        )
        .await;

        let instances = api()
            .search_process_instances(&connection(&server), "loan", "applicationId", "42", VariableValueType::Number)
            .await
            .expect("поиск");

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].business_key.as_deref(), Some("BR-42"));
        let requests = requests_to(&server, "/engine-rest/process-instance").await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "POST");
        assert!(query(&requests[0]).contains("maxResults=100"));
        let json = body_json(&requests[0]);
        assert_eq!(json["processDefinitionKey"], "loan");
        let variables = json["variables"].as_array().expect("массив переменных");
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0]["name"], "applicationId");
        assert_eq!(variables[0]["operator"], "eq");
        assert_eq!(variables[0]["value"], 42);
        assert!(variables[0]["value"].is_number());
        assert_eq!(json["sorting"][0]["sortBy"], "instanceId");
    }

    #[tokio::test]
    async fn automatic_variable_search_tries_string_and_detected_primitive_type() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/engine-rest/process-instance"))
            .and(|request: &Request| {
                serde_json::from_slice::<Value>(&request.body).map(|body| body["variables"][0]["value"].is_string()).unwrap_or(false)
            })
            .respond_with(json_response(
                200,
                r#"[{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false}]"#,
            ))
            .mount(&server)
            .await;
        mount(&server, "POST", "/engine-rest/process-instance", 200, "[]").await;

        let instances = api()
            .search_process_instances(&connection(&server), "loan", "applicationId", "42", VariableValueType::Auto)
            .await
            .expect("поиск");

        assert_eq!(requests_to(&server, "/engine-rest/process-instance").await.len(), 2);
        assert_eq!(instances.iter().map(|instance| instance.id.as_str()).collect::<Vec<_>>(), vec!["instance-1"]);
    }

    #[tokio::test]
    async fn invalid_typed_search_value_fails_before_request() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(json_response(200, "[]")).mount(&server).await;

        let number = api().search_process_instances(&connection(&server), "loan", "amount", "abc", VariableValueType::Number).await;
        let boolean = api().search_process_instances(&connection(&server), "loan", "flag", "yes", VariableValueType::Boolean).await;

        assert!(number.expect_err("число").starts_with("The variable value must be a valid number.\nRequest: "));
        assert!(boolean.expect_err("boolean").starts_with("A boolean value must be true or false.\nRequest: "));
        assert!(server.received_requests().await.unwrap_or_default().is_empty());
    }

    #[test]
    fn automatic_search_values_cover_every_plausible_type() {
        let render = |value: &str| {
            search_json_values(value, VariableValueType::Auto).expect("значения").iter().map(Value::to_string).collect::<Vec<_>>()
        };
        assert_eq!(render("42"), vec!["\"42\"", "42"]);
        assert_eq!(render("4.5"), vec!["\"4.5\"", "4.5"]);
        assert_eq!(render(" True "), vec!["\" True \"", "true"]);
        assert_eq!(render("APP-1"), vec!["\"APP-1\""]);
    }
}
