use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use futures_util::future::join_all;
use tokio::sync::Semaphore;

use super::json::{json_object, string};
use super::{ApiResult, CamundaApi, MAX_PARALLEL_HISTORY_REQUESTS};
use crate::camunda::bpmn::parse_bpmn;
use crate::camunda::http::{ApiError, encode};
use crate::models::*;

#[derive(Debug)]
pub(super) struct LoadedBpmn {
    pub(super) xml: String,
    pub(super) diagram: BpmnDiagram,
}

impl CamundaApi {
    // Диаграмма процесса кэшируется по подключению и ID определения; при переполнении кэш сбрасывается целиком.
    pub(super) async fn load_bpmn(&self, connection: &CamundaConnection, process_definition_id: &str) -> ApiResult<Arc<LoadedBpmn>> {
        let cache_key = format!("{}\n{process_definition_id}", connection.rest_url);
        let cached = self.bpmn_cache.lock().unwrap().get(&cache_key).cloned();
        if let Some(loaded) = cached {
            return Ok(loaded);
        }
        let url = format!("{}/process-definition/{}/xml", connection.rest_url, encode(process_definition_id));
        let body = json_object(&self.get(&url).await?)?;
        let xml = string(&body, "bpmn20Xml").unwrap_or_default();
        let diagram = parse_bpmn(&xml).map_err(ApiError::Other)?;
        let loaded = Arc::new(LoadedBpmn { xml, diagram });
        let mut cache = self.bpmn_cache.lock().unwrap();
        if cache.len() >= BPMN_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(cache_key, loaded.clone());
        Ok(loaded)
    }

    pub(super) async fn load_diagrams(
        &self,
        connection: &CamundaConnection,
        definition_ids: Vec<String>,
    ) -> HashMap<String, Arc<LoadedBpmn>> {
        let unique: Vec<String> = definition_ids.into_iter().collect::<BTreeSet<_>>().into_iter().collect();
        let semaphore = Semaphore::new(MAX_PARALLEL_HISTORY_REQUESTS);
        let loads = unique.into_iter().map(|definition_id| {
            let semaphore = &semaphore;
            async move {
                let _permit = semaphore.acquire().await.ok()?;
                let loaded = self.load_bpmn(connection, &definition_id).await.ok()?;
                Some((definition_id, loaded))
            }
        });
        join_all(loads).await.into_iter().flatten().collect()
    }
}

const BPMN_CACHE_LIMIT: usize = 16;
