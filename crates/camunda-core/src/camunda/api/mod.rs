mod activity_history;
mod bpmn_cache;
mod bulk_retry;
mod business_key;
mod current_activities;
mod dashboard;
mod definition_details;
mod definitions;
mod grouping;
mod incident_activities;
mod incidents;
mod instance_details;
mod instances;
mod jobs;
mod json;
mod metadata;
mod recovery;
mod search;
mod statistics;
mod teleport;
#[cfg(test)]
mod test_support;
mod variable_catalog;
mod variables;
mod waiting_activities;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::camunda::catalog::ProcessVariableCatalogRepository;
use crate::camunda::http::{self, ApiError};
use bpmn_cache::LoadedBpmn;
use variable_catalog::CachedVariableCatalog;

pub struct CamundaApi {
    client: reqwest::Client,
    catalog_repository: Arc<dyn ProcessVariableCatalogRepository>,
    now: Clock,
    bpmn_cache: Mutex<HashMap<String, Arc<LoadedBpmn>>>,
    variable_catalog_cache: Mutex<HashMap<String, CachedVariableCatalog>>,
}

const SEARCH_LIMIT: usize = 100;
const DETAIL_LIST_LIMIT: usize = 500;
const MAX_PARALLEL_HISTORY_REQUESTS: usize = 6;
const HISTORY_PAGE_SIZE: usize = 1_000;
const HISTORY_LIMIT: usize = 20_000;

type ApiResult<T> = Result<T, ApiError>;
type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

impl CamundaApi {
    pub fn new(client: reqwest::Client, catalog_repository: Arc<dyn ProcessVariableCatalogRepository>) -> Self {
        Self {
            client,
            catalog_repository,
            now: Arc::new(Utc::now),
            bpmn_cache: Mutex::new(HashMap::new()),
            variable_catalog_cache: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    pub fn with_clock(mut self, now: impl Fn() -> DateTime<Utc> + Send + Sync + 'static) -> Self {
        self.now = Arc::new(now);
        self
    }

    async fn mutate(&self, url: &str, method: reqwest::Method, body: Option<Value>) -> Result<(), String> {
        let request = self.client.request(method, url).header("Content-Type", "application/json");
        let request = match body {
            Some(body) => request.body(body.to_string()),
            None => request,
        };
        http::send(&self.client, request).await.map(|_| ()).map_err(|error| http::user_message(&error, url))
    }

    async fn get(&self, url: &str) -> ApiResult<String> {
        http::send(&self.client, self.client.get(url)).await
    }

    async fn post(&self, url: &str, body: Option<Value>) -> ApiResult<String> {
        let request = self.client.post(url).header("Content-Type", "application/json");
        let request = match body {
            Some(body) => request.body(body.to_string()),
            None => request,
        };
        http::send(&self.client, request).await
    }

    // Листает ответ страницами по HISTORY_PAGE_SIZE, пока страница не окажется неполной или не наберётся HISTORY_LIMIT.
    async fn load_paged<T>(&self, base_url: &str, parser: fn(&str) -> ApiResult<Vec<T>>) -> ApiResult<Vec<T>> {
        let mut values = Vec::new();
        let mut first_result = 0;
        while values.len() < HISTORY_LIMIT {
            let separator = if base_url.contains('?') { '&' } else { '?' };
            let page_url = format!("{base_url}{separator}firstResult={first_result}&maxResults={HISTORY_PAGE_SIZE}");
            let page = parser(&self.get(&page_url).await?)?;
            let page_size = page.len();
            values.extend(page);
            if page_size < HISTORY_PAGE_SIZE {
                return Ok(values);
            }
            first_result += page_size;
        }
        values.truncate(HISTORY_LIMIT);
        Ok(values)
    }
}
