use std::sync::Arc;

use camunda_support_lib::camunda::catalog::InMemoryProcessVariableCatalogRepository;
use camunda_support_lib::camunda::{connection_tester, http, CamundaApi};
use camunda_support_lib::models::*;

// Дымовая проверка API против локальной Camunda: cargo run --example smoke -- http://localhost:18080/engine-rest
#[tokio::main]
async fn main() {
    let rest_url = std::env::args().nth(1).unwrap_or_else(|| "http://localhost:18080/engine-rest".to_string());
    let client = http::camunda_client();
    println!("version: {:?}", connection_tester::test_connection(&client, &rest_url).await);
    let connection = CamundaConnection {
        id: "local".into(),
        name: "local".into(),
        rest_url,
        cockpit_url: String::new(),
        environment: Environment::Local,
        engine_version: None,
    };
    let api = CamundaApi::new(client, Arc::new(InMemoryProcessVariableCatalogRepository::default()));
    let dashboard = api.load_dashboard(&connection, &DashboardDateFilter::default()).await;
    match &dashboard {
        Ok(dashboard) => println!(
            "dashboard: {} definitions, timeline {} points, incidents types {}, waiting {}",
            dashboard.definitions.len(),
            dashboard.timeline.len(),
            dashboard.incident_types.len(),
            dashboard.longest_waiting_activities.len()
        ),
        Err(error) => println!("dashboard error: {error}"),
    }
    let today = api.load_dashboard(&connection, &DashboardDateFilter { preset: DashboardDatePreset::Today, from_date: String::new(), to_date: String::new() }).await;
    println!("today: {}", today.as_ref().map(|d| format!("{} completed, title {}", d.completed_instances, d.timeline_title)).unwrap_or_else(|e| e.clone()));
    let definitions = api.load_search_process_definitions(&connection).await.unwrap_or_default();
    println!("search definitions: {}", definitions.len());
    let Some(definition) = definitions.first() else { return };
    let catalog = api.load_process_variable_catalog(&connection, &definition.key).await;
    println!("catalog {}: {}", definition.key, catalog.as_ref().map(|c| format!("{} vars from {} instances", c.variables.len(), c.inspected_instance_count)).unwrap_or_else(|e| e.clone()));
    let details = api.load_process_definition_details(&connection, &definition.id, &DashboardDateFilter::default()).await;
    println!("definition details: {}", details.as_ref().map(|d| format!("{} instances", d.instances.len())).unwrap_or_else(|e| e.clone()));
    if let Ok(details) = &details {
        if let Some(item) = details.instances.first() {
            let instance = api.load_process_instance_details(&connection, &item.instance.id).await;
            println!("instance details: {}", instance.as_ref().map(|d| format!("{} vars, {} active, {} history, {} incidents, bpmn {} bytes, {} nodes", d.variables.len(), d.active_activities.len(), d.activity_history.len(), d.incidents.len(), d.bpmn_xml.len(), d.diagram.nodes.len())).unwrap_or_else(|e| e.clone()));
        }
    }
    let incidents = api.load_incidents(&connection, None, &DashboardDateFilter::default(), None).await;
    println!("incidents: {}", incidents.as_ref().map(|i| i.len().to_string()).unwrap_or_else(|e| e.clone()));
}
