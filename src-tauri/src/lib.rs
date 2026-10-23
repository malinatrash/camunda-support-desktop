pub mod camunda;
mod commands;
mod connection;
mod deep_link;
mod migration;
pub mod models;
mod release_stats;
mod storage;
mod updater;

use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use camunda::CamundaApi;
use models::DeepLinkEvent;
use release_stats::ReleaseDownloadStatsService;
use storage::Storage;
use updater::AppUpdateService;

pub struct AppState {
    pub storage: Arc<Storage>,
    pub api: CamundaApi,
    pub updater: AppUpdateService,
    pub stats: ReleaseDownloadStatsService,
    pub client: reqwest::Client,
    pub pending_deep_links: Mutex<Vec<DeepLinkEvent>>,
    pub version: String,
    pub build: String,
}

pub const DEEP_LINK_EVENT: &str = "deep-link";

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let version = app.package_info().version.to_string();
            let build = env!("CAMUNDA_SUPPORT_BUILD").to_string();
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("camunda-support"));
            let storage = Arc::new(Storage::new(data_dir));
            migrate_legacy_preferences(&storage);

            let client = camunda::http::camunda_client();
            let state = AppState {
                api: CamundaApi::new(client.clone(), storage.clone()),
                updater: AppUpdateService::new(&version),
                stats: ReleaseDownloadStatsService::new(&version),
                client,
                storage,
                pending_deep_links: Mutex::new(Vec::new()),
                version,
                build,
            };
            app.manage(state);

            apply_saved_theme(app.handle());
            register_deep_links(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_connections,
            commands::add_connection,
            commands::delete_connection,
            commands::validate_connection_draft,
            commands::derive_cockpit_url,
            commands::test_connection,
            commands::get_theme,
            commands::set_theme,
            commands::load_dashboard,
            commands::load_incidents,
            commands::load_process_definition_versions,
            commands::search_process_instances,
            commands::load_search_process_definitions,
            commands::load_process_variable_catalog,
            commands::invalidate_process_variable_catalog,
            commands::load_process_definition_details,
            commands::load_process_instance_details,
            commands::search_instances_by_business_key,
            commands::retry_incidents,
            commands::teleport_process_instance,
            commands::set_job_retries,
            commands::set_external_task_retries,
            commands::unlock_external_task,
            commands::set_process_instance_suspended,
            commands::update_process_variable,
            commands::check_for_update,
            commands::download_and_open_update,
            commands::load_release_download_stats,
            commands::parse_deep_link,
            commands::process_instance_deep_link,
            commands::take_pending_deep_links,
            commands::open_in_browser,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Один раз переносит подключения и тему из java.util.prefs старой Kotlin-версии.
fn migrate_legacy_preferences(storage: &Storage) {
    if storage.has_connections_file() {
        return;
    }
    let legacy = migration::read_legacy_preferences();
    if !legacy.connections.is_empty() {
        log::info!("перенесено подключений из старой версии: {}", legacy.connections.len());
        storage.replace_connections(legacy.connections);
    } else {
        storage.replace_connections(Vec::new());
    }
    if let (Some(theme), None) = (legacy.theme, storage.load_theme()) {
        storage.save_theme(theme);
    }
}

fn apply_saved_theme(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    let theme = app.state::<AppState>().storage.load_theme();
    let _ = window.set_theme(theme.map(commands::native_theme));
}

fn register_deep_links(app: &tauri::AppHandle) {
    #[cfg(any(windows, target_os = "linux"))]
    {
        if cfg!(debug_assertions) {
            let _ = app.deep_link().register_all();
        }
    }
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        enqueue_deep_links(app, urls.iter().map(|url| url.to_string()));
    }
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        enqueue_deep_links(&handle, event.urls().iter().map(|url| url.to_string()));
    });
}

// Кладёт разобранные ссылки в очередь и будит фронт; очередь вычитывается командой take_pending_deep_links.
pub fn enqueue_deep_links(app: &tauri::AppHandle, urls: impl Iterator<Item = String>) {
    let state = app.state::<AppState>();
    {
        let mut pending = state.pending_deep_links.lock().unwrap();
        for url in urls {
            pending.push(deep_link::parse_deep_link(&url));
        }
    }
    let _ = app.emit(DEEP_LINK_EVENT, ());
    focus_main_window(app);
}

fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
