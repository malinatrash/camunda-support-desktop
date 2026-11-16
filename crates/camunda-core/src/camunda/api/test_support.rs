pub(super) use std::collections::HashSet;
pub(super) use std::sync::Arc;

pub(super) use chrono::{DateTime, Utc};
pub(super) use serde_json::{Value, json};
pub(super) use wiremock::matchers::{any, method, path};
pub(super) use wiremock::{Mock, MockServer, Request, ResponseTemplate};

pub(super) use super::CamundaApi;
pub(super) use crate::camunda::catalog::{InMemoryProcessVariableCatalogRepository, ProcessVariableCatalogRepository};
pub(super) use crate::camunda::http;
pub(super) use crate::models::*;

pub(super) fn connection(server: &MockServer) -> CamundaConnection {
    CamundaConnection {
        id: "test".to_string(),
        name: "Test".to_string(),
        rest_url: format!("{}/engine-rest", server.uri()),
        cockpit_url: "http://127.0.0.1/camunda/app/cockpit".to_string(),
        environment: Environment::Test,
        engine_version: None,
    }
}

pub(super) fn api() -> CamundaApi {
    CamundaApi::new(http::camunda_client(), Arc::new(InMemoryProcessVariableCatalogRepository::default()))
}

pub(super) fn api_at(now: &str, repository: Arc<dyn ProcessVariableCatalogRepository>) -> CamundaApi {
    let fixed = DateTime::parse_from_rfc3339(now).expect("дата теста").with_timezone(&Utc);
    CamundaApi::new(http::camunda_client(), repository).with_clock(move || fixed)
}

pub(super) fn custom(from: &str, to: &str) -> DashboardDateFilter {
    DashboardDateFilter { preset: DashboardDatePreset::Custom, from_date: from.to_string(), to_date: to.to_string() }
}

pub(super) fn json_response(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_string(), "application/json")
}

pub(super) async fn mount(server: &MockServer, http_method: &str, route: &str, status: u16, body: &str) {
    Mock::given(method(http_method)).and(path(route)).respond_with(json_response(status, body)).mount(server).await;
}

pub(super) async fn requests_to(server: &MockServer, route: &str) -> Vec<Request> {
    server.received_requests().await.unwrap_or_default().into_iter().filter(|request| request.url.path() == route).collect()
}

pub(super) fn query(request: &Request) -> String {
    request.url.query().unwrap_or_default().to_string()
}

pub(super) fn body_json(request: &Request) -> Value {
    serde_json::from_slice(&request.body).expect("JSON тела запроса")
}

pub(super) fn has_completed_flag(request: &Request) -> bool {
    query(request).contains("completed=true")
}

pub(super) fn sum_instances(dashboard: &ProcessDashboard) -> i32 {
    dashboard.definitions.iter().map(|definition| definition.instances).sum()
}

pub(super) fn sum_incidents(dashboard: &ProcessDashboard) -> i32 {
    dashboard.definitions.iter().map(|definition| definition.incidents).sum()
}
