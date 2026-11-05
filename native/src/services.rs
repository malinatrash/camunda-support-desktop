use std::future::Future;
use std::sync::Arc;

use camunda_core::camunda::{http, CamundaApi};
use camunda_core::storage::Storage;
use gpui_kit::{App, Global};

const APP_IDENTIFIER: &str = "com.malinatrash.camundasupport";

pub struct Services {
    pub api: Arc<CamundaApi>,
    pub storage: Arc<Storage>,
    runtime: tokio::runtime::Runtime,
}

impl Global for Services {}

impl Services {
    // Хранилище то же, что у Tauri-версии: подключения и тема общие.
    pub fn new() -> Self {
        let root = dirs::data_dir().unwrap_or_else(std::env::temp_dir).join(APP_IDENTIFIER);
        let storage = Arc::new(Storage::new(root));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let client = runtime.block_on(async { http::camunda_client() });
        Self { api: Arc::new(CamundaApi::new(client, storage.clone())), storage, runtime }
    }
}

// Запрос к Camunda идёт на tokio, результат ждут из задачи GPUI.
pub fn request<T, F, C>(cx: &App, call: C) -> impl Future<Output = Result<T, String>> + use<T, F, C>
where
    T: Send + 'static,
    F: Future<Output = Result<T, String>> + Send + 'static,
    C: FnOnce(Arc<CamundaApi>) -> F,
{
    let services = cx.global::<Services>();
    let handle = services.runtime.spawn(call(services.api.clone()));
    async move { handle.await.map_err(|error| error.to_string())? }
}
