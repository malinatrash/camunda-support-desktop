use tauri::ipc::Channel;
use tauri::State;

use crate::models::*;
use crate::{connection, deep_link, AppState};

type Api<'a> = State<'a, AppState>;

#[tauri::command]
pub fn app_info(state: Api<'_>) -> AppInfo {
    AppInfo {
        version: state.version.clone(),
        build: state.build.clone(),
        platform: std::env::consts::OS.to_string(),
    }
}

#[tauri::command]
pub fn list_connections(state: Api<'_>) -> Vec<CamundaConnection> {
    state.storage.load_connections()
}

#[tauri::command]
pub fn add_connection(
    state: Api<'_>,
    draft: ConnectionDraft,
    engine_version: Option<String>,
) -> Result<CamundaConnection, String> {
    let validation = connection::validate_connection_draft(&draft);
    if !validation.is_valid() {
        return Err(validation
            .name_error
            .or(validation.rest_url_error)
            .or(validation.cockpit_url_error)
            .unwrap_or_else(|| "Подключение заполнено некорректно".into()));
    }
    let existing = state.storage.load_connections();
    let id = connection::connection_id(&draft.name, &existing);
    let connection = connection::draft_to_connection(&draft, id, engine_version);
    state.storage.save_connection(&connection);
    Ok(connection)
}

#[tauri::command]
pub fn delete_connection(state: Api<'_>, connection_id: String) {
    state.storage.delete_connection(&connection_id);
}

#[tauri::command]
pub fn validate_connection_draft(draft: ConnectionDraft) -> ConnectionValidation {
    connection::validate_connection_draft(&draft)
}

#[tauri::command]
pub fn derive_cockpit_url(rest_url: String) -> String {
    connection::derive_cockpit_url(&rest_url)
}

#[tauri::command]
pub async fn test_connection(state: Api<'_>, rest_url: String) -> Result<ConnectionTestSuccess, String> {
    crate::camunda::connection_tester::test_connection(&state.client, &rest_url).await
}

#[tauri::command]
pub fn get_theme(state: Api<'_>) -> AppThemeMode {
    state.storage.load_theme().unwrap_or(AppThemeMode::Light)
}

#[tauri::command]
pub fn set_theme(state: Api<'_>, window: tauri::WebviewWindow, theme: AppThemeMode) {
    state.storage.save_theme(theme);
    let _ = window.set_theme(Some(native_theme(theme)));
}

pub fn native_theme(theme: AppThemeMode) -> tauri::Theme {
    match theme {
        AppThemeMode::Light => tauri::Theme::Light,
        AppThemeMode::Dark => tauri::Theme::Dark,
    }
}

#[tauri::command]
pub async fn load_dashboard(
    state: Api<'_>,
    connection: CamundaConnection,
    date_filter: DashboardDateFilter,
) -> Result<ProcessDashboard, String> {
    state.api.load_dashboard(&connection, &date_filter).await
}

#[tauri::command]
pub async fn load_incidents(
    state: Api<'_>,
    connection: CamundaConnection,
    process_definition_id: Option<String>,
    date_filter: DashboardDateFilter,
    process_definition_key: Option<String>,
) -> Result<Vec<ProcessIncident>, String> {
    state
        .api
        .load_incidents(&connection, process_definition_id.as_deref(), &date_filter, process_definition_key.as_deref())
        .await
}

#[tauri::command]
pub async fn load_process_definition_versions(
    state: Api<'_>,
    connection: CamundaConnection,
    process_definition_key: String,
) -> Result<Vec<ProcessDefinitionSummary>, String> {
    state.api.load_process_definition_versions(&connection, &process_definition_key).await
}

#[tauri::command]
pub async fn search_process_instances(
    state: Api<'_>,
    connection: CamundaConnection,
    process_definition_key: String,
    variable_name: String,
    variable_value: String,
    value_type: VariableValueType,
) -> Result<Vec<ProcessInstanceSummary>, String> {
    state
        .api
        .search_process_instances(&connection, &process_definition_key, &variable_name, &variable_value, value_type)
        .await
}

#[tauri::command]
pub async fn load_search_process_definitions(
    state: Api<'_>,
    connection: CamundaConnection,
) -> Result<Vec<ProcessDefinitionSummary>, String> {
    state.api.load_search_process_definitions(&connection).await
}

#[tauri::command]
pub async fn load_process_variable_catalog(
    state: Api<'_>,
    connection: CamundaConnection,
    process_definition_key: String,
) -> Result<ProcessVariableCatalog, String> {
    state.api.load_process_variable_catalog(&connection, &process_definition_key).await
}

#[tauri::command]
pub fn invalidate_process_variable_catalog(state: Api<'_>, connection: CamundaConnection, process_definition_key: String) {
    state.api.invalidate_process_variable_catalog(&connection, &process_definition_key);
}

#[tauri::command]
pub async fn load_process_definition_details(
    state: Api<'_>,
    connection: CamundaConnection,
    process_definition_id: String,
    date_filter: DashboardDateFilter,
) -> Result<ProcessDefinitionDetails, String> {
    state.api.load_process_definition_details(&connection, &process_definition_id, &date_filter).await
}

#[tauri::command]
pub async fn load_process_instance_details(
    state: Api<'_>,
    connection: CamundaConnection,
    process_instance_id: String,
) -> Result<ProcessInstanceDetails, String> {
    state.api.load_process_instance_details(&connection, &process_instance_id).await
}

#[tauri::command]
pub async fn search_instances_by_business_key(
    state: Api<'_>,
    connection: CamundaConnection,
    query: String,
) -> Result<Vec<ProcessInstanceSummary>, String> {
    state.api.search_instances_by_business_key(&connection, &query).await
}

#[tauri::command]
pub async fn retry_incidents(
    state: Api<'_>,
    connection: CamundaConnection,
    incidents: Vec<ProcessIncident>,
    retries: i32,
) -> Result<BulkRetryResult, String> {
    state.api.retry_incidents(&connection, &incidents, retries).await
}

#[tauri::command]
pub async fn teleport_process_instance(
    state: Api<'_>,
    connection: CamundaConnection,
    process_instance_id: String,
    request: TeleportRequest,
) -> Result<(), String> {
    state.api.teleport_process_instance(&connection, &process_instance_id, &request).await
}

#[tauri::command]
pub async fn set_job_retries(state: Api<'_>, connection: CamundaConnection, job_id: String, retries: i32) -> Result<(), String> {
    state.api.set_job_retries(&connection, &job_id, retries).await
}

#[tauri::command]
pub async fn set_external_task_retries(
    state: Api<'_>,
    connection: CamundaConnection,
    external_task_id: String,
    retries: i32,
) -> Result<(), String> {
    state.api.set_external_task_retries(&connection, &external_task_id, retries).await
}

#[tauri::command]
pub async fn unlock_external_task(state: Api<'_>, connection: CamundaConnection, external_task_id: String) -> Result<(), String> {
    state.api.unlock_external_task(&connection, &external_task_id).await
}

#[tauri::command]
pub async fn set_process_instance_suspended(
    state: Api<'_>,
    connection: CamundaConnection,
    process_instance_id: String,
    suspended: bool,
) -> Result<(), String> {
    state.api.set_process_instance_suspended(&connection, &process_instance_id, suspended).await
}

#[tauri::command]
pub async fn update_process_variable(
    state: Api<'_>,
    connection: CamundaConnection,
    process_instance_id: String,
    update: ProcessVariableUpdate,
) -> Result<(), String> {
    state.api.update_process_variable(&connection, &process_instance_id, &update).await
}

#[tauri::command]
pub async fn check_for_update(state: Api<'_>) -> Result<UpdateCheckResult, String> {
    Ok(state.updater.check_for_update(&state.version).await)
}

#[tauri::command]
pub async fn download_and_open_update(
    state: Api<'_>,
    update: AppUpdate,
    on_progress: Channel<UpdateDownloadProgress>,
) -> Result<UpdateInstallResult, String> {
    let report = move |progress: UpdateDownloadProgress| {
        let _ = on_progress.send(progress);
    };
    Ok(state.updater.download_and_open(&update, &report).await)
}

#[tauri::command]
pub async fn load_release_download_stats(state: Api<'_>, force_refresh: bool) -> Result<ReleaseDownloadStats, String> {
    state.stats.load(force_refresh).await
}

#[tauri::command]
pub fn parse_deep_link(raw: String) -> DeepLinkEvent {
    deep_link::parse_deep_link(&raw)
}

#[tauri::command]
pub fn process_instance_deep_link(connection: CamundaConnection, process_instance_id: String) -> Option<String> {
    deep_link::process_instance_deep_link(&connection, &process_instance_id)
}

#[tauri::command]
pub fn take_pending_deep_links(state: Api<'_>) -> Vec<DeepLinkEvent> {
    std::mem::take(&mut *state.pending_deep_links.lock().unwrap())
}

#[tauri::command]
pub fn open_in_browser(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Можно открывать только http и https ссылки".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|error| error.to_string())
}
