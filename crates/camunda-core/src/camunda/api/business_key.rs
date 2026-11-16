use std::collections::HashSet;

use super::CamundaApi;
use super::instances::{parse_instance, parse_instances};
use super::json::json_object;
use crate::camunda::http::{self, ApiError, encode};
use crate::models::*;

impl CamundaApi {
    // Ищет заявки по бизнес-ключу среди запущенных и исторических; точный ID заявки тоже подходит.
    pub async fn search_instances_by_business_key(
        &self,
        connection: &CamundaConnection,
        query: &str,
    ) -> Result<Vec<ProcessInstanceSummary>, String> {
        let rest_url = &connection.rest_url;
        let needle = query.trim();
        if needle.is_empty() {
            return Err("Введите бизнес-ключ или ID заявки.".to_string());
        }
        let like = encode(&format!("%{needle}%"));
        let runtime_url = format!("{rest_url}/process-instance?businessKeyLike={like}&maxResults={QUICK_SEARCH_LIMIT}");
        let history_url = format!(
            "{rest_url}/history/process-instance?processInstanceBusinessKeyLike={like}&sortBy=startTime&sortOrder=desc&maxResults={QUICK_SEARCH_LIMIT}"
        );
        let by_id_url = format!("{rest_url}/history/process-instance/{}", encode(needle));
        let result = async {
            let runtime = async { parse_instances(&self.get(&runtime_url).await?) };
            let history = async { parse_instances(&self.get(&history_url).await?) };
            let by_id = async {
                match self.get(&by_id_url).await {
                    Ok(body) => json_object(&body).and_then(|value| parse_instance(&value)).ok(),
                    Err(_) => None,
                }
            };
            let (runtime, history, by_id) = tokio::join!(runtime, history, by_id);
            let mut seen = HashSet::new();
            let mut merged = Vec::new();
            for instance in by_id.into_iter().chain(runtime?).chain(history?) {
                if seen.insert(instance.id.clone()) {
                    merged.push(instance);
                }
            }
            Ok::<_, ApiError>(merged)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &runtime_url))
    }
}

const QUICK_SEARCH_LIMIT: usize = 50;

#[cfg(test)]
mod tests {
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn business_key_search_merges_runtime_history_and_exact_id() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance",
            200,
            r#"[{"id":"run-1","definitionId":"alpha:1:x","businessKey":"APP-10","suspended":false,"tenantId":null,"ended":false}]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[{"id":"run-1","processDefinitionId":"alpha:1:x","processDefinitionKey":"alpha","processDefinitionName":"Alpha","businessKey":"APP-10","startTime":"2026-10-23T10:00:00.000+0000","endTime":null,"state":"ACTIVE"},
                {"id":"hist-2","processDefinitionId":"alpha:1:x","processDefinitionKey":"alpha","processDefinitionName":"Alpha","businessKey":"APP-100","startTime":"2026-10-22T10:00:00.000+0000","endTime":"2026-10-22T11:00:00.000+0000","state":"COMPLETED"}]"#,
        )
        .await;
        mount(&server, "GET", "/engine-rest/history/process-instance/APP-10", 404, r#"{"message":"not found"}"#).await;

        let found = api().search_instances_by_business_key(&connection(&server), " APP-10 ").await.expect("поиск");

        assert_eq!(found.iter().map(|instance| instance.id.as_str()).collect::<Vec<_>>(), vec!["run-1", "hist-2"]);
        assert!(!found[0].ended);
        assert!(found[1].ended);
        assert_eq!(found[1].definition_name.as_deref(), Some("Alpha"));
        let runtime = requests_to(&server, "/engine-rest/process-instance").await;
        assert!(query(&runtime[0]).contains("businessKeyLike=%25APP-10%25"));
        assert!(api().search_instances_by_business_key(&connection(&server), "  ").await.is_err());
    }
}
