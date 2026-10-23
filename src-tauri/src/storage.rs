use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::camunda::catalog::{ProcessVariableCatalogRepository, StoredProcessVariableCatalog};
use crate::connection::derive_cockpit_url;
use crate::models::{AppThemeMode, CamundaConnection};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionsFile {
    #[serde(default)]
    connections: Vec<CamundaConnection>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsFile {
    #[serde(default)]
    theme: Option<AppThemeMode>,
}

// Локальное хранилище приложения: JSON-файлы в каталоге данных пользователя.
pub struct Storage {
    root: PathBuf,
    lock: Mutex<()>,
}

impl Storage {
    pub fn new(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root);
        let _ = fs::create_dir_all(root.join("variable-catalogs"));
        Self { root, lock: Mutex::new(()) }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn connections_path(&self) -> PathBuf {
        self.root.join("connections.json")
    }

    fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn has_connections_file(&self) -> bool {
        self.connections_path().exists()
    }

    pub fn load_connections(&self) -> Vec<CamundaConnection> {
        let _guard = self.lock.lock().unwrap();
        let mut file: ConnectionsFile = read_json(&self.connections_path()).unwrap_or_default();
        file.connections.retain(|connection| !connection.name.trim().is_empty() && !connection.rest_url.trim().is_empty());
        for connection in &mut file.connections {
            if connection.cockpit_url.trim().is_empty() {
                connection.cockpit_url = derive_cockpit_url(&connection.rest_url);
            }
        }
        file.connections.sort_by_key(|connection| connection.name.to_lowercase());
        file.connections
    }

    pub fn save_connection(&self, connection: &CamundaConnection) {
        let mut connections = self.load_connections();
        let _guard = self.lock.lock().unwrap();
        connections.retain(|existing| existing.id != connection.id);
        connections.push(connection.clone());
        write_json(&self.connections_path(), &ConnectionsFile { connections });
    }

    pub fn replace_connections(&self, connections: Vec<CamundaConnection>) {
        let _guard = self.lock.lock().unwrap();
        write_json(&self.connections_path(), &ConnectionsFile { connections });
    }

    pub fn delete_connection(&self, connection_id: &str) {
        let mut connections = self.load_connections();
        let _guard = self.lock.lock().unwrap();
        connections.retain(|existing| existing.id != connection_id);
        write_json(&self.connections_path(), &ConnectionsFile { connections });
    }

    pub fn load_theme(&self) -> Option<AppThemeMode> {
        let _guard = self.lock.lock().unwrap();
        read_json::<SettingsFile>(&self.settings_path()).and_then(|settings| settings.theme)
    }

    pub fn save_theme(&self, theme: AppThemeMode) {
        let _guard = self.lock.lock().unwrap();
        let mut settings: SettingsFile = read_json(&self.settings_path()).unwrap_or_default();
        settings.theme = Some(theme);
        write_json(&self.settings_path(), &settings);
    }

    fn catalog_path(&self, connection_rest_url: &str, process_definition_key: &str) -> PathBuf {
        let digest = Sha256::digest(format!("{connection_rest_url}\n{process_definition_key}").as_bytes());
        let name: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        self.root.join("variable-catalogs").join(format!("{name}.json"))
    }
}

impl ProcessVariableCatalogRepository for Storage {
    fn load(&self, connection_rest_url: &str, process_definition_key: &str) -> Option<StoredProcessVariableCatalog> {
        let stored: StoredProcessVariableCatalog = read_json(&self.catalog_path(connection_rest_url, process_definition_key))?;
        (stored.catalog.process_definition_key == process_definition_key).then_some(stored)
    }

    fn save(&self, connection_rest_url: &str, stored: &StoredProcessVariableCatalog) {
        write_json(&self.catalog_path(connection_rest_url, &stored.catalog.process_definition_key), stored);
    }

    fn remove(&self, connection_rest_url: &str, process_definition_key: &str) {
        let _ = fs::remove_file(self.catalog_path(connection_rest_url, process_definition_key));
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

// Пишет JSON атомарно: сначала во временный файл рядом, затем переименование.
fn write_json<T: Serialize>(path: &Path, value: &T) {
    let Ok(text) = serde_json::to_string_pretty(value) else { return };
    let temporary = path.with_extension("json.tmp");
    if fs::write(&temporary, text).is_ok() {
        let _ = fs::rename(&temporary, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Environment, ProcessVariableCatalog, ProcessVariableDescriptor};

    fn connection(id: &str, name: &str) -> CamundaConnection {
        CamundaConnection {
            id: id.into(),
            name: name.into(),
            rest_url: "https://h/engine-rest".into(),
            cockpit_url: String::new(),
            environment: Environment::Test,
            engine_version: Some("7.17.0".into()),
        }
    }

    #[test]
    fn stores_connections_sorted_and_with_derived_cockpit() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage::new(directory.path().to_path_buf());
        storage.save_connection(&connection("b", "Бета"));
        storage.save_connection(&connection("a", "Альфа"));
        let loaded = storage.load_connections();
        assert_eq!(loaded.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["a", "b"]);
        assert_eq!(loaded[0].cockpit_url, "https://h/camunda/app/cockpit");
        storage.delete_connection("a");
        assert_eq!(storage.load_connections().len(), 1);
    }

    #[test]
    fn stores_theme_and_catalogs() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage::new(directory.path().to_path_buf());
        assert!(storage.load_theme().is_none());
        storage.save_theme(AppThemeMode::Dark);
        assert_eq!(storage.load_theme(), Some(AppThemeMode::Dark));

        let stored = StoredProcessVariableCatalog {
            catalog: ProcessVariableCatalog {
                process_definition_key: "dover".into(),
                variables: vec![ProcessVariableDescriptor { name: "iin".into(), types: vec!["String".into()], occurrences: 3 }],
                inspected_instance_count: 3,
                instances_truncated: false,
            },
            loaded_at_epoch_millis: 42,
        };
        storage.save("https://h/engine-rest", &stored);
        assert_eq!(storage.load("https://h/engine-rest", "dover"), Some(stored));
        assert!(storage.load("https://other/engine-rest", "dover").is_none());
        storage.remove("https://h/engine-rest", "dover");
        assert!(storage.load("https://h/engine-rest", "dover").is_none());
    }
}
