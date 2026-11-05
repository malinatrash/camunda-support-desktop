use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::models::ProcessVariableCatalog;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredProcessVariableCatalog {
    pub catalog: ProcessVariableCatalog,
    pub loaded_at_epoch_millis: i64,
}

pub trait ProcessVariableCatalogRepository: Send + Sync {
    fn load(&self, connection_rest_url: &str, process_definition_key: &str) -> Option<StoredProcessVariableCatalog>;
    fn save(&self, connection_rest_url: &str, stored: &StoredProcessVariableCatalog);
    fn remove(&self, connection_rest_url: &str, process_definition_key: &str);
}

#[derive(Default)]
pub struct InMemoryProcessVariableCatalogRepository {
    catalogs: Mutex<HashMap<(String, String), StoredProcessVariableCatalog>>,
}

impl ProcessVariableCatalogRepository for InMemoryProcessVariableCatalogRepository {
    fn load(&self, connection_rest_url: &str, process_definition_key: &str) -> Option<StoredProcessVariableCatalog> {
        self.catalogs
            .lock()
            .unwrap()
            .get(&(connection_rest_url.to_string(), process_definition_key.to_string()))
            .cloned()
    }

    fn save(&self, connection_rest_url: &str, stored: &StoredProcessVariableCatalog) {
        self.catalogs.lock().unwrap().insert(
            (connection_rest_url.to_string(), stored.catalog.process_definition_key.clone()),
            stored.clone(),
        );
    }

    fn remove(&self, connection_rest_url: &str, process_definition_key: &str) {
        self.catalogs
            .lock()
            .unwrap()
            .remove(&(connection_rest_url.to_string(), process_definition_key.to_string()));
    }
}
