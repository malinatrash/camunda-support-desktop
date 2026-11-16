mod build;
mod linux;
mod macos;
mod windows;

use std::collections::HashMap;

pub use build::build_legacy_preferences;
pub use linux::parse_linux_prefs_xml;
pub use macos::parse_macos_plist_json;
pub use windows::{decode_windows_name, encode_windows_name};

use crate::models::{AppThemeMode, CamundaConnection};
use crate::storage::Storage;
#[cfg(target_os = "linux")]
use linux::read_legacy_nodes;
#[cfg(target_os = "macos")]
use macos::read_legacy_nodes;
#[cfg(windows)]
use windows::read_legacy_nodes;

// Один раз переносит подключения и тему из java.util.prefs старой Kotlin-версии.
pub fn migrate_legacy_preferences(storage: &Storage) {
    if storage.has_connections_file() {
        return;
    }
    let legacy = read_legacy_preferences();
    if !legacy.connections.is_empty() {
        log::info!("перенесено подключений из старой версии: {}", legacy.connections.len());
    }
    storage.replace_connections(legacy.connections);
    if let (Some(theme), None) = (legacy.theme, storage.load_theme()) {
        storage.save_theme(theme);
    }
}

// Читает подключения и тему старой версии (java.util.prefs) на текущей ОС; при любой проблеме — пусто.
pub fn read_legacy_preferences() -> LegacyPreferences {
    build_legacy_preferences(read_legacy_nodes().unwrap_or_default())
}

// Данные старой Kotlin-версии, прочитанные из java.util.prefs этого пользователя.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LegacyPreferences {
    pub connections: Vec<CamundaConnection>,
    pub theme: Option<AppThemeMode>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LegacyNodes {
    pub connections: Vec<HashMap<String, String>>,
    pub theme: Option<String>,
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn read_legacy_nodes() -> Option<LegacyNodes> {
    None
}
