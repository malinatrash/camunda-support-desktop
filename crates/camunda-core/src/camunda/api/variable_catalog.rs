use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use tokio::sync::Semaphore;

use super::grouping::group_in_order;
use super::instances::parse_instances;
use super::json::{json_array, string};
use super::{ApiResult, CamundaApi, MAX_PARALLEL_HISTORY_REQUESTS, SEARCH_LIMIT};
use crate::camunda::catalog::StoredProcessVariableCatalog;
use crate::camunda::http::{self, ApiError, encode};
use crate::models::*;

impl CamundaApi {
    pub async fn load_process_variable_catalog(
        &self,
        connection: &CamundaConnection,
        process_definition_key: &str,
    ) -> Result<ProcessVariableCatalog, String> {
        let normalized_key = process_definition_key.trim().to_string();
        let cache_key = format!("{}\n{normalized_key}", connection.rest_url);
        let now = (self.now)();
        if let Some(catalog) = self.cached_variable_catalog(&cache_key, now) {
            return Ok(catalog);
        }
        if let Some(persisted) = self.catalog_repository.load(&connection.rest_url, &normalized_key) {
            let persisted_at = DateTime::from_timestamp_millis(persisted.loaded_at_epoch_millis);
            if let Some(persisted_at) = persisted_at.filter(|at| (now - at).num_milliseconds() < VARIABLE_CATALOG_CACHE_TTL_MILLIS) {
                self.store_variable_catalog(cache_key, persisted.catalog.clone(), persisted_at);
                return Ok(persisted.catalog);
            }
        }

        let instances_url = format!(
            "{}/process-instance?processDefinitionKey={}&firstResult=0&maxResults={}",
            connection.rest_url,
            encode(&normalized_key),
            VARIABLE_CATALOG_INSTANCE_LIMIT + 1
        );
        let result = async {
            let loaded_instances = parse_instances(&self.get(&instances_url).await?)?;
            let instances_truncated = loaded_instances.len() > VARIABLE_CATALOG_INSTANCE_LIMIT;
            let instances: Vec<ProcessInstanceSummary> = loaded_instances.into_iter().take(VARIABLE_CATALOG_INSTANCE_LIMIT).collect();
            let semaphore = Semaphore::new(MAX_PARALLEL_HISTORY_REQUESTS);
            let batches = instances.chunks(VARIABLE_INSTANCE_ID_BATCH_SIZE).map(|batch| {
                let semaphore = &semaphore;
                async move {
                    let _permit =
                        semaphore.acquire().await.map_err(|_| ApiError::Other(rust_i18n::t!("core.http.semaphore_closed").to_string()))?;
                    let ids = encode(&batch.iter().map(|instance| instance.id.as_str()).collect::<Vec<_>>().join(","));
                    let base_url = format!(
                        "{}/variable-instance?processInstanceIdIn={ids}&deserializeValues=false&sortBy=variableName&sortOrder=asc",
                        connection.rest_url
                    );
                    self.load_paged(&base_url, parse_variable_descriptors).await
                }
            });
            let samples: Vec<VariableDescriptorSample> = try_join_all(batches).await?.into_iter().flatten().collect();
            let mut descriptors: Vec<ProcessVariableDescriptor> = group_in_order(samples, |sample| sample.name.clone())
                .into_iter()
                .map(|(name, samples)| {
                    let types: BTreeSet<String> = samples.iter().map(|sample| sample.value_type.clone()).collect();
                    ProcessVariableDescriptor { name, types: types.into_iter().collect(), occurrences: samples.len() as i32 }
                })
                .collect();
            descriptors.sort_by_cached_key(|descriptor| descriptor.name.to_lowercase());
            let catalog = ProcessVariableCatalog {
                process_definition_key: normalized_key.clone(),
                variables: descriptors,
                inspected_instance_count: instances.len() as i32,
                instances_truncated,
            };
            let loaded_at = (self.now)();
            self.store_variable_catalog(cache_key.clone(), catalog.clone(), loaded_at);
            self.catalog_repository.save(
                &connection.rest_url,
                &StoredProcessVariableCatalog { catalog: catalog.clone(), loaded_at_epoch_millis: loaded_at.timestamp_millis() },
            );
            Ok(catalog)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &instances_url))
    }

    pub fn invalidate_process_variable_catalog(&self, connection: &CamundaConnection, process_definition_key: &str) {
        let normalized_key = process_definition_key.trim();
        self.variable_catalog_cache.lock().unwrap().remove(&format!("{}\n{normalized_key}", connection.rest_url));
        self.catalog_repository.remove(&connection.rest_url, normalized_key);
    }

    fn cached_variable_catalog(&self, cache_key: &str, now: DateTime<Utc>) -> Option<ProcessVariableCatalog> {
        let cache = self.variable_catalog_cache.lock().unwrap();
        cache
            .get(cache_key)
            .filter(|cached| (now - cached.loaded_at).num_milliseconds() < VARIABLE_CATALOG_CACHE_TTL_MILLIS)
            .map(|cached| cached.catalog.clone())
    }

    fn store_variable_catalog(&self, cache_key: String, catalog: ProcessVariableCatalog, loaded_at: DateTime<Utc>) {
        let mut cache = self.variable_catalog_cache.lock().unwrap();
        if cache.len() >= VARIABLE_CATALOG_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(cache_key, CachedVariableCatalog { catalog, loaded_at });
    }
}

pub(super) struct CachedVariableCatalog {
    pub(super) catalog: ProcessVariableCatalog,
    pub(super) loaded_at: DateTime<Utc>,
}

struct VariableDescriptorSample {
    name: String,
    value_type: String,
}

fn parse_variable_descriptors(body: &str) -> ApiResult<Vec<VariableDescriptorSample>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(VariableDescriptorSample {
                name: string(value, "name")?,
                value_type: string(value, "type").unwrap_or_else(|| rust_i18n::t!("core.variables.unknown_type").to_string()),
            })
        })
        .collect())
}

const VARIABLE_CATALOG_CACHE_LIMIT: usize = 64;
const VARIABLE_CATALOG_INSTANCE_LIMIT: usize = SEARCH_LIMIT;
const VARIABLE_INSTANCE_ID_BATCH_SIZE: usize = 40;
const VARIABLE_CATALOG_CACHE_TTL_MILLIS: i64 = 24 * 60 * 60 * 1_000;

#[cfg(test)]
mod tests {
    use crate::camunda::api::test_support::*;

    #[tokio::test]
    async fn search_catalog_loads_processes_and_concrete_variables_from_their_active_instances() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[
                {"id":"loan:3:def","key":"loan","name":"Кредит","version":3,"deploymentId":"dep","suspended":false},
                {"id":"pledge:1:def","key":"pledge","name":"Залог","version":1,"deploymentId":"dep","suspended":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance",
            200,
            r#"[
                {"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false},
                {"id":"instance-2","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/variable-instance",
            200,
            r#"[
                {"id":"v1","name":"applicationId","type":"String","processInstanceId":"instance-1"},
                {"id":"v2","name":"applicationId","type":"String","processInstanceId":"instance-2"},
                {"id":"v3","name":"amount","type":"Long","processInstanceId":"instance-2"}
            ]"#,
        )
        .await;
        let connection = connection(&server);
        let repository: Arc<dyn ProcessVariableCatalogRepository> = Arc::new(InMemoryProcessVariableCatalogRepository::default());
        let api = api_at("2026-07-24T00:00:00Z", repository.clone());

        let definitions = api.load_search_process_definitions(&connection).await.expect("процессы");
        let catalog = api.load_process_variable_catalog(&connection, "loan").await.expect("каталог");

        assert_eq!(definitions.iter().map(|definition| definition.display_name()).collect::<Vec<_>>(), vec!["Залог", "Кредит"]);
        assert!(query(&requests_to(&server, "/engine-rest/process-definition").await[0]).contains("latestVersion=true"));
        let instance_query = query(&requests_to(&server, "/engine-rest/process-instance").await[0]);
        assert!(instance_query.contains("processDefinitionKey=loan"), "{instance_query}");
        assert!(instance_query.contains("maxResults=101"), "{instance_query}");
        let variable_query = query(&requests_to(&server, "/engine-rest/variable-instance").await[0]);
        assert!(variable_query.contains("processInstanceIdIn=instance-1%2Cinstance-2"), "{variable_query}");
        assert!(variable_query.contains("deserializeValues=false"), "{variable_query}");
        assert_eq!(catalog.process_definition_key, "loan");
        assert_eq!(catalog.inspected_instance_count, 2);
        assert!(!catalog.instances_truncated);
        assert_eq!(catalog.variables.iter().map(|variable| variable.name.as_str()).collect::<Vec<_>>(), vec!["amount", "applicationId"]);
        assert_eq!(catalog.variables[1].occurrences, 2);
        assert_eq!(catalog.variables[0].types, vec!["Long".to_string()]);

        api.load_process_variable_catalog(&connection, "loan").await.expect("каталог из кэша");
        assert_eq!(
            requests_to(&server, "/engine-rest/variable-instance").await.len(),
            1,
            "Повторное открытие процесса должно использовать кэш каталога"
        );

        api_at("2026-07-24T01:00:00Z", repository.clone())
            .load_process_variable_catalog(&connection, "loan")
            .await
            .expect("каталог с диска");
        assert_eq!(
            requests_to(&server, "/engine-rest/variable-instance").await.len(),
            1,
            "Новый сеанс приложения должен использовать дисковый кэш каталога"
        );

        api_at("2026-07-25T00:00:01Z", repository.clone())
            .load_process_variable_catalog(&connection, "loan")
            .await
            .expect("каталог после TTL");
        assert_eq!(requests_to(&server, "/engine-rest/variable-instance").await.len(), 2, "Просроченный кэш перечитывается");

        api.invalidate_process_variable_catalog(&connection, "loan");
        assert!(repository.load(&connection.rest_url, "loan").is_none());
        api.load_process_variable_catalog(&connection, " loan ").await.expect("каталог после сброса");
        assert_eq!(requests_to(&server, "/engine-rest/variable-instance").await.len(), 3);
    }
}
