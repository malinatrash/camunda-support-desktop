use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use futures_util::future::{join_all, try_join_all};
use futures_util::FutureExt;
use serde_json::{json, Value};
use tokio::sync::Semaphore;

use crate::camunda::bpmn::parse_bpmn;
use crate::camunda::catalog::{ProcessVariableCatalogRepository, StoredProcessVariableCatalog};
use crate::camunda::dates::{self, CamundaDateRange};
use crate::camunda::http::{self, encode, ApiError};
use crate::models::*;

const SEARCH_LIMIT: usize = 100;
const SEARCH_DEFINITION_LIMIT: usize = 500;
const DETAIL_LIST_LIMIT: usize = 500;
const QUICK_SEARCH_LIMIT: usize = 50;
const DASHBOARD_INCIDENT_LIMIT: usize = 500;
const BPMN_CACHE_LIMIT: usize = 16;
const VARIABLE_CATALOG_CACHE_LIMIT: usize = 64;
const VARIABLE_CATALOG_INSTANCE_LIMIT: usize = SEARCH_LIMIT;
const VARIABLE_INSTANCE_ID_BATCH_SIZE: usize = 40;
const MAX_PARALLEL_HISTORY_REQUESTS: usize = 6;
const HISTORY_PAGE_SIZE: usize = 1_000;
const HISTORY_CHART_LIMIT: usize = 20_000;
const DASHBOARD_ACTIVITY_LIMIT: usize = 12;
const DASHBOARD_WAITING_ACTIVITY_LIMIT: usize = 6;
const VARIABLE_CATALOG_CACHE_TTL_MILLIS: i64 = 24 * 60 * 60 * 1_000;
const ENDED_HISTORY_STATES: &[&str] = &["COMPLETED", "EXTERNALLY_TERMINATED", "INTERNALLY_TERMINATED"];

type ApiResult<T> = Result<T, ApiError>;
type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

pub struct CamundaApi {
    client: reqwest::Client,
    catalog_repository: Arc<dyn ProcessVariableCatalogRepository>,
    now: Clock,
    bpmn_cache: Mutex<HashMap<String, Arc<LoadedBpmn>>>,
    variable_catalog_cache: Mutex<HashMap<String, CachedVariableCatalog>>,
}

#[derive(Debug, Default, Clone, Copy)]
struct DefinitionStatistics {
    instances: i32,
    incidents: i32,
}

#[derive(Debug, Clone)]
struct ProcessIncidentWithInstance {
    process_instance_id: String,
    incident: ProcessIncident,
}

#[derive(Debug, Clone)]
struct HistoricActivityInstance {
    id: String,
    activity_id: String,
    activity_name: Option<String>,
    activity_type: String,
    process_definition_id: Option<String>,
    process_instance_id: Option<String>,
    parent_activity_instance_id: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    canceled: bool,
}

struct WaitingActivitySample<'a> {
    activity: &'a HistoricActivityInstance,
    process_instance_id: &'a str,
    waiting_millis: i64,
}

#[derive(Debug)]
struct LoadedBpmn {
    xml: String,
    diagram: BpmnDiagram,
}

struct VariableDescriptorSample {
    name: String,
    value_type: String,
}

struct CachedVariableCatalog {
    catalog: ProcessVariableCatalog,
    loaded_at: DateTime<Utc>,
}

#[derive(Debug)]
struct PagedResult<T> {
    values: Vec<T>,
    truncated: bool,
}

impl<T> Default for PagedResult<T> {
    fn default() -> Self {
        Self { values: Vec::new(), truncated: false }
    }
}

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

    pub async fn load_dashboard(
        &self,
        connection: &CamundaConnection,
        date_filter: &DashboardDateFilter,
    ) -> Result<ProcessDashboard, String> {
        let definitions_url =
            format!("{}/process-definition?latestVersion=true&sortBy=deployTime&sortOrder=desc", connection.rest_url);
        self.dashboard(connection, date_filter, &definitions_url)
            .await
            .map_err(|error| http::user_message(&error, &definitions_url))
    }

    async fn dashboard(
        &self,
        connection: &CamundaConnection,
        date_filter: &DashboardDateFilter,
        definitions_url: &str,
    ) -> ApiResult<ProcessDashboard> {
        let rest_url = &connection.rest_url;
        let range = dates::dashboard_range(date_filter).map_err(ApiError::Invalid)?;
        let live = date_filter.is_live();
        let statistics_url = format!("{rest_url}/process-definition/statistics?incidents=true");
        let completed_count_url = format!(
            "{rest_url}/history/process-instance/count?completed=true&finishedAfter={}&finishedBefore={}",
            encode(&range.after),
            encode(&range.before)
        );

        let definitions = async { parse_definitions(&self.get(definitions_url).await?) }.shared();
        let statistics = async {
            if live {
                parse_statistics(&self.get(&statistics_url).await?)
            } else {
                let definitions = definitions.clone().await?;
                self.load_historic_statistics(connection, &definitions, date_filter).await
            }
        };
        let started = async { self.load_historic_instances(connection, &range, false).await.unwrap_or_default() };
        let completed = async { self.load_historic_instances(connection, &range, true).await.unwrap_or_default() };
        let historic_incidents =
            async { self.load_historic_incidents(connection, &range, None, None).await.unwrap_or_default() };
        let runtime_incidents = async {
            if live {
                Some(self.load_runtime_incidents(connection, None, None).await.unwrap_or_default())
            } else {
                None
            }
        };
        let unfinished_activities = async { self.load_unfinished_activities(connection).await.unwrap_or_default() };
        let completed_count =
            async { self.get(&completed_count_url).await.ok().and_then(|body| parse_count(&body).ok()) };

        let (definitions, statistics, started, completed, historic_incidents, runtime_incidents, unfinished, completed_count) = tokio::join!(
            definitions.clone(),
            statistics,
            started,
            completed,
            historic_incidents,
            runtime_incidents,
            unfinished_activities,
            completed_count
        );
        let definitions = definitions?;
        let statistics = statistics?;

        let incidents_for_breakdown = runtime_incidents.unwrap_or_else(|| historic_incidents.values.clone());
        let mut attention_incidents: Vec<ProcessIncident> = incidents_for_breakdown.iter().take(DASHBOARD_INCIDENT_LIMIT).cloned().collect();
        attention_incidents.sort_by_cached_key(|incident| {
            std::cmp::Reverse(dates::parse_camunda_date_time(incident.timestamp.as_deref()).map(|value| value.with_timezone(&Utc)))
        });
        let started_ids: HashSet<&str> = started.values.iter().map(|instance| instance.id.as_str()).collect();
        let waiting_candidates: Vec<&HistoricActivityInstance> = unfinished
            .values
            .iter()
            .filter(|activity| activity.process_instance_id.as_deref().is_some_and(|id| started_ids.contains(id)))
            .collect();
        let (incident_activities, longest_waiting_activities) = tokio::join!(
            self.load_incident_activities(connection, &incidents_for_breakdown),
            self.load_longest_waiting_activities(connection, &waiting_candidates)
        );

        let mut completed_durations: Vec<i64> = completed
            .values
            .iter()
            .filter_map(|instance| dates::duration_millis(instance.start_time.as_deref(), instance.end_time.as_deref()))
            .collect();
        completed_durations.sort_unstable();
        let mut incident_types: Vec<DashboardIncidentType> =
            count_in_order(incidents_for_breakdown.iter().map(|incident| incident.incident_type.clone()))
                .into_iter()
                .map(|(incident_type, count)| DashboardIncidentType { incident_type, count })
                .collect();
        incident_types.sort_by(|left, right| right.count.cmp(&left.count));

        Ok(ProcessDashboard {
            definitions: definitions
                .into_iter()
                .map(|definition| {
                    let counts = statistics.get(&definition.id).copied().unwrap_or_default();
                    ProcessDefinitionSummary { instances: counts.instances, incidents: counts.incidents, ..definition }
                })
                .collect(),
            timeline: build_timeline(&range, &started.values, &completed.values, &historic_incidents.values),
            incident_types,
            incident_activities,
            longest_waiting_activities,
            duration_buckets: build_duration_buckets(&completed_durations),
            timeline_title: dates::timeline_title(date_filter),
            average_duration_millis: average(&completed_durations),
            median_duration_millis: percentile(&completed_durations, 0.50),
            p95_duration_millis: percentile(&completed_durations, 0.95),
            completed_instances: completed_count.unwrap_or(completed.values.len() as i32),
            timeline_truncated: started.truncated || completed.truncated || historic_incidents.truncated || unfinished.truncated,
            incidents: attention_incidents,
        })
    }

    pub async fn load_incidents(
        &self,
        connection: &CamundaConnection,
        process_definition_id: Option<&str>,
        date_filter: &DashboardDateFilter,
        process_definition_key: Option<&str>,
    ) -> Result<Vec<ProcessIncident>, String> {
        let diagnostic_url = format!("{}/incident", connection.rest_url);
        let result = async {
            let mut incidents = if date_filter.is_live() {
                self.load_runtime_incidents(connection, process_definition_id, process_definition_key).await?
            } else {
                let range = dates::camunda_date_range(date_filter).map_err(ApiError::Invalid)?;
                self.load_historic_incidents(connection, &range, process_definition_id, process_definition_key).await?.values
            };
            incidents.sort_by_cached_key(|incident| {
                std::cmp::Reverse(dates::parse_camunda_date_time(incident.timestamp.as_deref()).map(|value| value.with_timezone(&Utc)))
            });
            Ok(incidents)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &diagnostic_url))
    }

    pub async fn load_process_definition_versions(
        &self,
        connection: &CamundaConnection,
        process_definition_key: &str,
    ) -> Result<Vec<ProcessDefinitionSummary>, String> {
        let url = format!(
            "{}/process-definition?key={}&sortBy=version&sortOrder=desc",
            connection.rest_url,
            encode(process_definition_key)
        );
        let result = async { parse_definitions(&self.get(&url).await?) }.await;
        result.map_err(|error| http::user_message(&error, &url))
    }

    pub async fn search_process_instances(
        &self,
        connection: &CamundaConnection,
        process_definition_key: &str,
        variable_name: &str,
        variable_value: &str,
        value_type: VariableValueType,
    ) -> Result<Vec<ProcessInstanceSummary>, String> {
        let url = format!("{}/process-instance?firstResult=0&maxResults={SEARCH_LIMIT}", connection.rest_url);
        let result = async {
            let url = &url;
            let searches = search_json_values(variable_value, value_type)?.into_iter().map(|typed_value| {
                let body = json!({
                    "processDefinitionKey": process_definition_key,
                    "variables": [{ "name": variable_name.trim(), "operator": "eq", "value": typed_value }],
                    "sorting": [{ "sortBy": "instanceId", "sortOrder": "desc" }],
                });
                async move { parse_instances(&self.post(url, Some(body)).await?) }
            });
            let pages = try_join_all(searches).await?;
            let mut seen = HashSet::new();
            Ok(pages
                .into_iter()
                .flatten()
                .filter(|instance| seen.insert(instance.id.clone()))
                .take(SEARCH_LIMIT)
                .collect())
        }
        .await;
        result.map_err(|error| http::user_message(&error, &url))
    }

    pub async fn load_search_process_definitions(
        &self,
        connection: &CamundaConnection,
    ) -> Result<Vec<ProcessDefinitionSummary>, String> {
        let url = format!(
            "{}/process-definition?latestVersion=true&sortBy=key&sortOrder=asc&maxResults={SEARCH_DEFINITION_LIMIT}",
            connection.rest_url
        );
        let result = async {
            let mut seen = HashSet::new();
            let mut definitions: Vec<ProcessDefinitionSummary> = parse_definitions(&self.get(&url).await?)?
                .into_iter()
                .filter(|definition| seen.insert((definition.key.clone(), definition.tenant_id.clone())))
                .collect();
            definitions.sort_by_cached_key(|definition| definition.display_name().to_lowercase());
            Ok(definitions)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &url))
    }

    pub async fn load_process_variable_catalog(
        &self,
        connection: &CamundaConnection,
        process_definition_key: &str,
    ) -> Result<ProcessVariableCatalog, String> {
        let normalized_key = process_definition_key.trim().to_string();
        let cache_key = format!("{}\n{normalized_key}", connection.rest_url);
        let now = (self.now)();
        if let Some(catalog) = self.cached_variable_catalog(&cache_key, now) {
            return Ok(catalog);
        }
        if let Some(persisted) = self.catalog_repository.load(&connection.rest_url, &normalized_key) {
            let persisted_at = DateTime::from_timestamp_millis(persisted.loaded_at_epoch_millis);
            if let Some(persisted_at) = persisted_at.filter(|at| (now - at).num_milliseconds() < VARIABLE_CATALOG_CACHE_TTL_MILLIS) {
                self.store_variable_catalog(cache_key, persisted.catalog.clone(), persisted_at);
                return Ok(persisted.catalog);
            }
        }

        let instances_url = format!(
            "{}/process-instance?processDefinitionKey={}&firstResult=0&maxResults={}",
            connection.rest_url,
            encode(&normalized_key),
            VARIABLE_CATALOG_INSTANCE_LIMIT + 1
        );
        let result = async {
            let loaded_instances = parse_instances(&self.get(&instances_url).await?)?;
            let instances_truncated = loaded_instances.len() > VARIABLE_CATALOG_INSTANCE_LIMIT;
            let instances: Vec<ProcessInstanceSummary> =
                loaded_instances.into_iter().take(VARIABLE_CATALOG_INSTANCE_LIMIT).collect();
            let semaphore = Semaphore::new(MAX_PARALLEL_HISTORY_REQUESTS);
            let batches = instances.chunks(VARIABLE_INSTANCE_ID_BATCH_SIZE).map(|batch| {
                let semaphore = &semaphore;
                async move {
                    let _permit = semaphore.acquire().await.map_err(|_| ApiError::Other("семафор закрыт".to_string()))?;
                    let ids = encode(&batch.iter().map(|instance| instance.id.as_str()).collect::<Vec<_>>().join(","));
                    let base_url = format!(
                        "{}/variable-instance?processInstanceIdIn={ids}&deserializeValues=false&sortBy=variableName&sortOrder=asc",
                        connection.rest_url
                    );
                    Ok::<_, ApiError>(self.load_paged(&base_url, parse_variable_descriptors).await?.values)
                }
            });
            let samples: Vec<VariableDescriptorSample> = try_join_all(batches).await?.into_iter().flatten().collect();
            let mut descriptors: Vec<ProcessVariableDescriptor> = group_in_order(samples, |sample| sample.name.clone())
                .into_iter()
                .map(|(name, samples)| {
                    let types: BTreeSet<String> = samples.iter().map(|sample| sample.value_type.clone()).collect();
                    ProcessVariableDescriptor { name, types: types.into_iter().collect(), occurrences: samples.len() as i32 }
                })
                .collect();
            descriptors.sort_by_cached_key(|descriptor| descriptor.name.to_lowercase());
            let catalog = ProcessVariableCatalog {
                process_definition_key: normalized_key.clone(),
                variables: descriptors,
                inspected_instance_count: instances.len() as i32,
                instances_truncated,
            };
            let loaded_at = (self.now)();
            self.store_variable_catalog(cache_key.clone(), catalog.clone(), loaded_at);
            self.catalog_repository.save(
                &connection.rest_url,
                &StoredProcessVariableCatalog { catalog: catalog.clone(), loaded_at_epoch_millis: loaded_at.timestamp_millis() },
            );
            Ok(catalog)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &instances_url))
    }

    fn cached_variable_catalog(&self, cache_key: &str, now: DateTime<Utc>) -> Option<ProcessVariableCatalog> {
        let cache = self.variable_catalog_cache.lock().unwrap();
        cache
            .get(cache_key)
            .filter(|cached| (now - cached.loaded_at).num_milliseconds() < VARIABLE_CATALOG_CACHE_TTL_MILLIS)
            .map(|cached| cached.catalog.clone())
    }

    fn store_variable_catalog(&self, cache_key: String, catalog: ProcessVariableCatalog, loaded_at: DateTime<Utc>) {
        let mut cache = self.variable_catalog_cache.lock().unwrap();
        if cache.len() >= VARIABLE_CATALOG_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(cache_key, CachedVariableCatalog { catalog, loaded_at });
    }

    pub fn invalidate_process_variable_catalog(&self, connection: &CamundaConnection, process_definition_key: &str) {
        let normalized_key = process_definition_key.trim();
        self.variable_catalog_cache.lock().unwrap().remove(&format!("{}\n{normalized_key}", connection.rest_url));
        self.catalog_repository.remove(&connection.rest_url, normalized_key);
    }

    pub async fn load_process_definition_details(
        &self,
        connection: &CamundaConnection,
        process_definition_id: &str,
        date_filter: &DashboardDateFilter,
    ) -> Result<ProcessDefinitionDetails, String> {
        let rest_url = &connection.rest_url;
        let path_id = encode(process_definition_id);
        let query_id = encode(process_definition_id);
        let definition_url = format!("{rest_url}/process-definition/{path_id}");
        let result = async {
            let range = if date_filter.is_live() {
                None
            } else {
                Some(dates::camunda_date_range(date_filter).map_err(ApiError::Invalid)?)
            };
            let instances_url = match &range {
                None => format!(
                    "{rest_url}/process-instance?processDefinitionId={query_id}&sortBy=instanceId&sortOrder=desc&maxResults={DETAIL_LIST_LIMIT}"
                ),
                Some(range) => format!(
                    "{rest_url}/history/process-instance?processDefinitionId={query_id}&startedAfter={}&startedBefore={}&sortBy=startTime&sortOrder=desc&maxResults={DETAIL_LIST_LIMIT}",
                    encode(&range.after),
                    encode(&range.before)
                ),
            };
            let incidents_url = match &range {
                None => format!("{rest_url}/incident?processDefinitionId={query_id}&maxResults={DETAIL_LIST_LIMIT}"),
                Some(range) => format!(
                    "{rest_url}/history/incident?processDefinitionId={query_id}&createTimeAfter={}&createTimeBefore={}&sortBy=createTime&sortOrder=desc&maxResults={DETAIL_LIST_LIMIT}",
                    encode(&range.after),
                    encode(&range.before)
                ),
            };
            let (definition_body, instances_body, incidents_body) =
                tokio::try_join!(self.get(&definition_url), self.get(&instances_url), self.get(&incidents_url))?;
            let definition_json = json_object(&definition_body)?;
            let definition = parse_definition(&definition_json)?;
            let instances = parse_instances(&instances_body)?;
            let incidents = parse_incidents(&incidents_body)?;
            let incidents_by_instance: HashMap<&str, i32> =
                count_in_order(incidents.iter().map(|incident| incident.process_instance_id.as_str())).into_iter().collect();
            Ok(ProcessDefinitionDetails {
                definition: ProcessDefinitionSummary {
                    instances: instances.len() as i32,
                    incidents: incidents.len() as i32,
                    ..definition
                },
                instances: instances
                    .into_iter()
                    .map(|instance| {
                        let incident_count = incidents_by_instance.get(instance.id.as_str()).copied().unwrap_or(0);
                        RuntimeInstanceListItem { instance, incident_count }
                    })
                    .collect(),
                metadata: definition_metadata(&definition_json),
                date_filter: date_filter.clone(),
            })
        }
        .await;
        result.map_err(|error| http::user_message(&error, &definition_url))
    }

    pub async fn load_process_instance_details(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
    ) -> Result<ProcessInstanceDetails, String> {
        let rest_url = &connection.rest_url;
        let path_id = encode(process_instance_id);
        let query_id = encode(process_instance_id);
        let instance_url = format!("{rest_url}/process-instance/{path_id}");
        let result = async {
            let instance_json = json_object(&self.get(&instance_url).await?)?;
            let instance = parse_instance(&instance_json)?;
            let variables = async {
                parse_variables(&self.get(&format!("{rest_url}/process-instance/{path_id}/variables?deserializeValues=false")).await?)
            };
            let activities =
                async { parse_activity_tree(&self.get(&format!("{rest_url}/process-instance/{path_id}/activity-instances")).await?) };
            let activity_history = async {
                let history = self
                    .load_paged(
                        &format!("{rest_url}/history/activity-instance?processInstanceId={query_id}&sortBy=startTime&sortOrder=asc"),
                        parse_historic_activity_instances,
                    )
                    .await?;
                Ok::<_, ApiError>((to_activity_execution_summaries(&history.values), to_activity_timeline(&history.values)))
            };
            let incidents = async {
                let incidents =
                    parse_incidents(&self.get(&format!("{rest_url}/incident?processInstanceId={query_id}&maxResults={DETAIL_LIST_LIMIT}")).await?)?;
                Ok::<_, ApiError>(incidents.into_iter().map(|incident| incident.incident).collect::<Vec<_>>())
            };
            let jobs = async {
                parse_jobs(
                    &self.get(&format!("{rest_url}/job?processInstanceId={query_id}&withException=true&maxResults={DETAIL_LIST_LIMIT}")).await?,
                )
            };
            let external_tasks = async {
                parse_external_tasks(
                    &self.get(&format!("{rest_url}/external-task?processInstanceId={query_id}&maxResults={DETAIL_LIST_LIMIT}")).await?,
                )
            };
            let bpmn = self.load_bpmn(connection, &instance.definition_id);
            let (variables, activities, (activity_history, activity_timeline), incidents, jobs, external_tasks, bpmn) =
                tokio::try_join!(variables, activities, activity_history, incidents, jobs, external_tasks, bpmn)?;
            let active_activities =
                build_current_activities(activities, &activity_history, &incidents, &jobs, &external_tasks, &bpmn.diagram);
            Ok(ProcessInstanceDetails {
                instance,
                variables,
                active_activities,
                activity_history,
                incidents,
                jobs,
                external_tasks,
                diagram: bpmn.diagram.clone(),
                bpmn_xml: bpmn.xml.clone(),
                metadata: instance_metadata(&instance_json),
                activity_timeline,
            })
        }
        .await;
        result.map_err(|error| http::user_message(&error, &instance_url))
    }

    // Ищет заявки по бизнес-ключу среди запущенных и исторических; точный ID заявки тоже подходит.
    pub async fn search_instances_by_business_key(
        &self,
        connection: &CamundaConnection,
        query: &str,
    ) -> Result<Vec<ProcessInstanceSummary>, String> {
        let rest_url = &connection.rest_url;
        let needle = query.trim();
        if needle.is_empty() {
            return Err("Введите бизнес-ключ или ID заявки.".to_string());
        }
        let like = encode(&format!("%{needle}%"));
        let runtime_url = format!("{rest_url}/process-instance?businessKeyLike={like}&maxResults={QUICK_SEARCH_LIMIT}");
        let history_url = format!(
            "{rest_url}/history/process-instance?processInstanceBusinessKeyLike={like}&sortBy=startTime&sortOrder=desc&maxResults={QUICK_SEARCH_LIMIT}"
        );
        let by_id_url = format!("{rest_url}/history/process-instance/{}", encode(needle));
        let result = async {
            let runtime = async { parse_instances(&self.get(&runtime_url).await?) };
            let history = async { parse_instances(&self.get(&history_url).await?) };
            let by_id = async {
                match self.get(&by_id_url).await {
                    Ok(body) => json_object(&body).and_then(|value| parse_instance(&value)).ok(),
                    Err(_) => None,
                }
            };
            let (runtime, history, by_id) = tokio::join!(runtime, history, by_id);
            let mut seen = HashSet::new();
            let mut merged = Vec::new();
            for instance in by_id.into_iter().chain(runtime?).chain(history?) {
                if seen.insert(instance.id.clone()) {
                    merged.push(instance);
                }
            }
            Ok::<_, ApiError>(merged)
        }
        .await;
        result.map_err(|error| http::user_message(&error, &runtime_url))
    }

    // Массово выставляет retries заданиям и внешним задачам из списка инцидентов одним запросом на тип.
    pub async fn retry_incidents(
        &self,
        connection: &CamundaConnection,
        incidents: &[ProcessIncident],
        retries: i32,
    ) -> Result<BulkRetryResult, String> {
        let mut job_ids: Vec<&str> = Vec::new();
        let mut external_task_ids: Vec<&str> = Vec::new();
        let mut skipped = 0;
        for incident in incidents {
            match (incident.incident_type.as_str(), incident.configuration.as_deref().filter(|id| !id.trim().is_empty())) {
                ("failedJob", Some(id)) => job_ids.push(id),
                ("failedExternalTask", Some(id)) => external_task_ids.push(id),
                _ => skipped += 1,
            }
        }
        job_ids.sort_unstable();
        job_ids.dedup();
        external_task_ids.sort_unstable();
        external_task_ids.dedup();
        if job_ids.is_empty() && external_task_ids.is_empty() {
            return Err("Среди выбранных инцидентов нет заданий или внешних задач для повтора.".to_string());
        }
        if !job_ids.is_empty() {
            let url = format!("{}/job/retries", connection.rest_url);
            self.mutate(&url, reqwest::Method::PUT, Some(json!({ "jobIds": job_ids, "retries": retries }))).await?;
        }
        if !external_task_ids.is_empty() {
            let url = format!("{}/external-task/retries", connection.rest_url);
            self.mutate(&url, reqwest::Method::PUT, Some(json!({ "externalTaskIds": external_task_ids, "retries": retries }))).await?;
        }
        Ok(BulkRetryResult { jobs: job_ids.len() as i32, external_tasks: external_task_ids.len() as i32, skipped })
    }

    pub async fn teleport_process_instance(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
        request: &TeleportRequest,
    ) -> Result<(), String> {
        let url = format!("{}/process-instance/{}/modification", connection.rest_url, encode(process_instance_id));
        let body = json!({
            "skipCustomListeners": request.skip_custom_listeners,
            "skipIoMappings": request.skip_io_mappings,
            "annotation": request.annotation,
            "instructions": [
                { "type": "cancel", "activityInstanceId": request.source_activity_instance_id },
                { "type": "startBeforeActivity", "activityId": request.target_activity_id },
            ],
        });
        self.mutate(&url, reqwest::Method::POST, Some(body)).await
    }

    pub async fn set_job_retries(&self, connection: &CamundaConnection, job_id: &str, retries: i32) -> Result<(), String> {
        let url = format!("{}/job/{}/retries", connection.rest_url, encode(job_id));
        self.mutate(&url, reqwest::Method::PUT, Some(json!({ "retries": retries }))).await
    }

    pub async fn set_external_task_retries(
        &self,
        connection: &CamundaConnection,
        external_task_id: &str,
        retries: i32,
    ) -> Result<(), String> {
        let url = format!("{}/external-task/{}/retries", connection.rest_url, encode(external_task_id));
        self.mutate(&url, reqwest::Method::PUT, Some(json!({ "retries": retries }))).await
    }

    pub async fn unlock_external_task(&self, connection: &CamundaConnection, external_task_id: &str) -> Result<(), String> {
        let url = format!("{}/external-task/{}/unlock", connection.rest_url, encode(external_task_id));
        self.mutate(&url, reqwest::Method::POST, None).await
    }

    pub async fn set_process_instance_suspended(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
        suspended: bool,
    ) -> Result<(), String> {
        let url = format!("{}/process-instance/{}/suspended", connection.rest_url, encode(process_instance_id));
        self.mutate(&url, reqwest::Method::PUT, Some(json!({ "suspended": suspended }))).await
    }

    // Недопустимое значение переменной отклоняется до обращения к сети.
    pub async fn update_process_variable(
        &self,
        connection: &CamundaConnection,
        process_instance_id: &str,
        update: &ProcessVariableUpdate,
    ) -> Result<(), String> {
        let url = format!(
            "{}/process-instance/{}/variables/{}",
            connection.rest_url,
            encode(process_instance_id),
            encode(&update.name)
        );
        let value = variable_json_value(update)?;
        let value_info = match update.value_info.as_deref() {
            None => json!({}),
            Some(raw) => match serde_json::from_str::<Value>(raw) {
                Ok(parsed @ Value::Object(_)) => parsed,
                Ok(_) => json!({}),
                Err(_) => return Err("Поле valueInfo должно содержать корректный JSON-объект.".to_string()),
            },
        };
        let body = json!({ "value": value, "type": update.value_type, "valueInfo": value_info });
        self.mutate(&url, reqwest::Method::PUT, Some(body)).await
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

    // Диаграмма процесса кэшируется по подключению и ID определения; при переполнении кэш сбрасывается целиком.
    async fn load_bpmn(&self, connection: &CamundaConnection, process_definition_id: &str) -> ApiResult<Arc<LoadedBpmn>> {
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

    async fn load_diagrams(&self, connection: &CamundaConnection, definition_ids: Vec<String>) -> HashMap<String, Arc<LoadedBpmn>> {
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

    async fn load_incident_activities(
        &self,
        connection: &CamundaConnection,
        incidents: &[ProcessIncident],
    ) -> Vec<DashboardIncidentActivity> {
        let keys = incidents.iter().filter_map(|incident| {
            let activity_id = incident.failed_activity_id.as_ref().or(incident.activity_id.as_ref())?;
            Some((activity_id.clone(), incident.process_definition_id.clone()))
        });
        let mut aggregated = count_in_order(keys);
        aggregated.sort_by(|left, right| right.1.cmp(&left.1));
        aggregated.truncate(DASHBOARD_ACTIVITY_LIMIT);
        let definition_ids = aggregated.iter().filter_map(|((_, definition_id), _)| definition_id.clone()).collect();
        let diagrams = self.load_diagrams(connection, definition_ids).await;
        aggregated
            .into_iter()
            .map(|((activity_id, process_definition_id), count)| {
                let node = process_definition_id
                    .as_ref()
                    .and_then(|definition_id| diagrams.get(definition_id))
                    .and_then(|loaded| loaded.diagram.node(&activity_id));
                DashboardIncidentActivity {
                    activity_id,
                    process_definition_id,
                    count,
                    activity_name: node.and_then(|node| node.name.clone()),
                    topic: node.and_then(|node| node.topic.clone()),
                }
            })
            .collect()
    }

    async fn load_unfinished_activities(&self, connection: &CamundaConnection) -> ApiResult<PagedResult<HistoricActivityInstance>> {
        let base_url = format!("{}/history/activity-instance?unfinished=true&sortBy=startTime&sortOrder=asc", connection.rest_url);
        self.load_paged(&base_url, parse_historic_activity_instances).await
    }

    // Среди незавершённых активностей выбирает листовые (не родители) и считает ожидание до текущего момента.
    async fn load_longest_waiting_activities(
        &self,
        connection: &CamundaConnection,
        activities: &[&HistoricActivityInstance],
    ) -> Vec<DashboardWaitingActivity> {
        let parent_ids: HashSet<&str> =
            activities.iter().filter_map(|activity| activity.parent_activity_instance_id.as_deref()).collect();
        let current = (self.now)();
        let samples = activities.iter().filter(|activity| !parent_ids.contains(activity.id.as_str()) && activity.activity_type != "processDefinition").filter_map(|activity| {
            activity.process_definition_id.as_deref()?;
            let process_instance_id = activity.process_instance_id.as_deref()?;
            let start = dates::parse_camunda_date_time(activity.start_time.as_deref())?;
            let waiting_millis = (current - start.with_timezone(&Utc)).num_milliseconds().max(0);
            Some(WaitingActivitySample { activity, process_instance_id, waiting_millis })
        });
        let grouped = group_in_order(samples, |sample| {
            (sample.activity.process_definition_id.clone().unwrap_or_default(), sample.activity.activity_id.clone())
        });
        let mut aggregated: Vec<DashboardWaitingActivity> = grouped
            .into_iter()
            .filter_map(|((process_definition_id, activity_id), samples)| {
                let oldest = samples.iter().rev().max_by_key(|sample| sample.waiting_millis)?;
                let instance_ids: HashSet<&str> = samples.iter().map(|sample| sample.process_instance_id).collect();
                let total_waiting: i64 = samples.iter().map(|sample| sample.waiting_millis).sum();
                Some(DashboardWaitingActivity {
                    activity_id,
                    activity_name: samples
                        .iter()
                        .find_map(|sample| sample.activity.activity_name.as_deref().filter(|name| !name.trim().is_empty()))
                        .map(str::to_string),
                    activity_type: oldest.activity.activity_type.clone(),
                    topic: None,
                    process_definition_id,
                    oldest_process_instance_id: oldest.process_instance_id.to_string(),
                    instance_count: instance_ids.len() as i32,
                    average_waiting_millis: (total_waiting as f64 / samples.len() as f64) as i64,
                    maximum_waiting_millis: oldest.waiting_millis,
                    oldest_started_at: oldest.activity.start_time.clone(),
                })
            })
            .collect();
        aggregated.sort_by(|left, right| right.maximum_waiting_millis.cmp(&left.maximum_waiting_millis));
        aggregated.truncate(DASHBOARD_WAITING_ACTIVITY_LIMIT);

        let diagrams =
            self.load_diagrams(connection, aggregated.iter().map(|activity| activity.process_definition_id.clone()).collect()).await;
        aggregated
            .into_iter()
            .map(|activity| {
                let node = diagrams.get(&activity.process_definition_id).and_then(|loaded| loaded.diagram.node(&activity.activity_id));
                DashboardWaitingActivity {
                    activity_name: activity.activity_name.or_else(|| node.and_then(|node| node.name.clone())),
                    topic: node.and_then(|node| node.topic.clone()),
                    ..activity
                }
            })
            .collect()
    }

    async fn load_historic_statistics(
        &self,
        connection: &CamundaConnection,
        definitions: &[ProcessDefinitionSummary],
        date_filter: &DashboardDateFilter,
    ) -> ApiResult<HashMap<String, DefinitionStatistics>> {
        let range = dates::camunda_date_range(date_filter).map_err(ApiError::Invalid)?;
        let after = encode(&range.after);
        let before = encode(&range.before);
        let semaphore = Semaphore::new(MAX_PARALLEL_HISTORY_REQUESTS);
        let loads = definitions.iter().map(|definition| {
            let (semaphore, after, before) = (&semaphore, &after, &before);
            async move {
                let _permit = semaphore.acquire().await.map_err(|_| ApiError::Other("семафор закрыт".to_string()))?;
                let definition_id = encode(&definition.id);
                let instances_url = format!(
                    "{}/history/process-instance/count?processDefinitionId={definition_id}&startedAfter={after}&startedBefore={before}",
                    connection.rest_url
                );
                let incidents_url = format!(
                    "{}/history/incident/count?processDefinitionId={definition_id}&createTimeAfter={after}&createTimeBefore={before}",
                    connection.rest_url
                );
                let statistics = DefinitionStatistics {
                    instances: parse_count(&self.get(&instances_url).await?)?,
                    incidents: parse_count(&self.get(&incidents_url).await?)?,
                };
                Ok::<_, ApiError>((definition.id.clone(), statistics))
            }
        });
        Ok(try_join_all(loads).await?.into_iter().collect())
    }

    async fn load_historic_instances(
        &self,
        connection: &CamundaConnection,
        range: &CamundaDateRange,
        completed: bool,
    ) -> ApiResult<PagedResult<ProcessInstanceSummary>> {
        let time_filter = if completed {
            format!("completed=true&finishedAfter={}&finishedBefore={}&sortBy=endTime", encode(&range.after), encode(&range.before))
        } else {
            format!("startedAfter={}&startedBefore={}&sortBy=startTime", encode(&range.after), encode(&range.before))
        };
        let base_url = format!("{}/history/process-instance?{time_filter}&sortOrder=asc", connection.rest_url);
        self.load_paged(&base_url, parse_instances).await
    }

    async fn load_historic_incidents(
        &self,
        connection: &CamundaConnection,
        range: &CamundaDateRange,
        process_definition_id: Option<&str>,
        process_definition_key: Option<&str>,
    ) -> ApiResult<PagedResult<ProcessIncident>> {
        let definition_filter = process_definition_id.map(|id| format!("&processDefinitionId={}", encode(id))).unwrap_or_default();
        let key_filter = process_definition_key.map(|key| format!("&processDefinitionKeyIn={}", encode(key))).unwrap_or_default();
        let base_url = format!(
            "{}/history/incident?createTimeAfter={}&createTimeBefore={}{definition_filter}{key_filter}&sortBy=createTime&sortOrder=asc",
            connection.rest_url,
            encode(&range.after),
            encode(&range.before)
        );
        self.load_paged(&base_url, parse_plain_incidents).await
    }

    async fn load_runtime_incidents(
        &self,
        connection: &CamundaConnection,
        process_definition_id: Option<&str>,
        process_definition_key: Option<&str>,
    ) -> ApiResult<Vec<ProcessIncident>> {
        let filters: Vec<String> = [
            process_definition_id.map(|id| format!("processDefinitionId={}", encode(id))),
            process_definition_key.map(|key| format!("processDefinitionKeyIn={}", encode(key))),
        ]
        .into_iter()
        .flatten()
        .collect();
        let filter = if filters.is_empty() { String::new() } else { format!("?{}", filters.join("&")) };
        let base_url = format!("{}/incident{filter}", connection.rest_url);
        Ok(self.load_paged(&base_url, parse_plain_incidents).await?.values)
    }

    // Листает ответ страницами по HISTORY_PAGE_SIZE, пока страница не окажется неполной или не наберётся HISTORY_CHART_LIMIT.
    async fn load_paged<T>(&self, base_url: &str, parser: fn(&str) -> ApiResult<Vec<T>>) -> ApiResult<PagedResult<T>> {
        let mut values = Vec::new();
        let mut first_result = 0;
        while values.len() < HISTORY_CHART_LIMIT {
            let separator = if base_url.contains('?') { '&' } else { '?' };
            let page_url = format!("{base_url}{separator}firstResult={first_result}&maxResults={HISTORY_PAGE_SIZE}");
            let page = parser(&self.get(&page_url).await?)?;
            let page_size = page.len();
            values.extend(page);
            if page_size < HISTORY_PAGE_SIZE {
                return Ok(PagedResult { values, truncated: false });
            }
            first_result += page_size;
        }
        values.truncate(HISTORY_CHART_LIMIT);
        Ok(PagedResult { values, truncated: true })
    }
}

fn parse_definitions(body: &str) -> ApiResult<Vec<ProcessDefinitionSummary>> {
    json_array(body)?.iter().map(parse_definition).collect()
}

fn parse_definition(value: &Value) -> ApiResult<ProcessDefinitionSummary> {
    Ok(ProcessDefinitionSummary {
        id: string(value, "id").ok_or_else(|| ApiError::Invalid("Camunda не вернула ID process definition".to_string()))?,
        key: string(value, "key").ok_or_else(|| ApiError::Invalid("Camunda не вернула key process definition".to_string()))?,
        name: string(value, "name"),
        version: int(value, "version"),
        tenant_id: string(value, "tenantId"),
        suspended: boolean(value, "suspended"),
        deployment_id: string(value, "deploymentId"),
        instances: 0,
        incidents: 0,
        version_tag: string(value, "versionTag"),
        category: string(value, "category"),
        description: string(value, "description"),
        resource: string(value, "resource"),
        diagram_resource: string(value, "diagram"),
        history_time_to_live: nullable_int(value, "historyTimeToLive"),
        startable_in_tasklist: nullable_boolean(value, "startableInTasklist").unwrap_or(true),
    })
}

fn parse_statistics(body: &str) -> ApiResult<HashMap<String, DefinitionStatistics>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            let id = string(value, "id")?;
            let incidents = as_array(value.get("incidents")).iter().map(|incident| int(incident, "incidentCount")).sum();
            Some((id, DefinitionStatistics { instances: int(value, "instances"), incidents }))
        })
        .collect())
}

fn parse_instances(body: &str) -> ApiResult<Vec<ProcessInstanceSummary>> {
    Ok(json_array(body)?.iter().filter_map(|value| parse_instance(value).ok()).collect())
}

fn parse_instance(value: &Value) -> ApiResult<ProcessInstanceSummary> {
    let id = string(value, "id").ok_or_else(|| ApiError::Invalid("Camunda не вернула ID process instance".to_string()))?;
    let definition_id = string(value, "definitionId")
        .or_else(|| string(value, "processDefinitionId"))
        .ok_or_else(|| ApiError::Invalid("Camunda не вернула definitionId".to_string()))?;
    let state = string(value, "state");
    let end_time = string(value, "endTime");
    let ended_by_state = state.as_deref().is_some_and(|state| ENDED_HISTORY_STATES.contains(&state));
    Ok(ProcessInstanceSummary {
        definition_key: string(value, "definitionKey")
            .or_else(|| string(value, "processDefinitionKey"))
            .unwrap_or_else(|| definition_id.split(':').next().unwrap_or_default().to_string()),
        id,
        definition_id,
        business_key: string(value, "businessKey"),
        suspended: boolean(value, "suspended"),
        tenant_id: string(value, "tenantId"),
        ended: boolean(value, "ended") || end_time.is_some() || ended_by_state,
        start_time: string(value, "startTime"),
        end_time,
        state,
        definition_name: string(value, "processDefinitionName"),
    })
}

fn definition_metadata(value: &Value) -> Vec<ProcessMetadataField> {
    vec![
        metadata_field("ID определения", value, "id"),
        metadata_field("Ключ", value, "key"),
        metadata_field("Название", value, "name"),
        metadata_field("Описание", value, "description"),
        metadata_field("Версия", value, "version"),
        metadata_field("Метка версии", value, "versionTag"),
        metadata_field("Категория", value, "category"),
        metadata_field("Ресурс BPMN", value, "resource"),
        metadata_field("Ресурс диаграммы", value, "diagram"),
        metadata_field("ID развёртывания", value, "deploymentId"),
        metadata_field("ID тенанта", value, "tenantId"),
        metadata_field("Срок хранения истории", value, "historyTimeToLive"),
        metadata_field("Доступен для запуска в списке задач", value, "startableInTasklist"),
        metadata_field("Приостановлен", value, "suspended"),
    ]
}

fn instance_metadata(value: &Value) -> Vec<ProcessMetadataField> {
    vec![
        metadata_field("ID экземпляра", value, "id"),
        metadata_field("ID определения", value, "definitionId"),
        metadata_field("Ключ определения", value, "definitionKey"),
        metadata_field("Бизнес-ключ", value, "businessKey"),
        metadata_field("ID экземпляра кейса", value, "caseInstanceId"),
        metadata_field("ID тенанта", value, "tenantId"),
        metadata_field("Завершён", value, "ended"),
        metadata_field("Приостановлен", value, "suspended"),
    ]
}

fn metadata_field(label: &str, value: &Value, key: &str) -> ProcessMetadataField {
    ProcessMetadataField { label: label.to_string(), value: string(value, key).unwrap_or_else(|| "—".to_string()) }
}

fn parse_variables(body: &str) -> ApiResult<Vec<ProcessVariable>> {
    let mut variables: Vec<ProcessVariable> = json_object(body)?
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, value)| ProcessVariable {
            name: name.clone(),
            value_type: string(value, "type").unwrap_or_else(|| "Неизвестный".to_string()),
            value: match value.get("value") {
                None | Some(Value::Null) => "null".to_string(),
                Some(Value::String(text)) => text.clone(),
                Some(Value::Number(number)) => number.to_string(),
                Some(Value::Bool(flag)) => flag.to_string(),
                Some(other) => other.to_string(),
            },
            value_info: value.get("valueInfo").map(Value::to_string).filter(|info| info != "{}"),
        })
        .collect();
    variables.sort_by_cached_key(|variable| variable.name.to_lowercase());
    Ok(variables)
}

fn parse_variable_descriptors(body: &str) -> ApiResult<Vec<VariableDescriptorSample>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(VariableDescriptorSample {
                name: string(value, "name")?,
                value_type: string(value, "type").unwrap_or_else(|| "Неизвестный".to_string()),
            })
        })
        .collect())
}

fn parse_activity_tree(body: &str) -> ApiResult<Vec<ActiveActivityInstance>> {
    let mut result = Vec::new();
    visit_activity(&json_object(body)?, true, &mut result);
    Ok(result)
}

fn visit_activity(value: &Value, root: bool, result: &mut Vec<ActiveActivityInstance>) {
    let activity_type = string(value, "activityType").unwrap_or_else(|| "unknown".to_string());
    if !root && activity_type != "processDefinition" {
        result.push(ActiveActivityInstance {
            id: string(value, "id").unwrap_or_default(),
            activity_id: string(value, "activityId").unwrap_or_default(),
            activity_name: string(value, "activityName").or_else(|| string(value, "name")),
            activity_type,
            execution_ids: string_array(value, "executionIds"),
            incident_ids: string_array(value, "incidentIds"),
            evidence: vec![CurrentActivityEvidence::RuntimeActivity],
            cancellable: true,
        });
    }
    for child in as_array(value.get("childActivityInstances")) {
        visit_activity(child, false, result);
    }
    for transition in as_array(value.get("childTransitionInstances")) {
        result.push(ActiveActivityInstance {
            id: string(transition, "id").unwrap_or_default(),
            activity_id: string(transition, "activityId").unwrap_or_default(),
            activity_name: string(transition, "activityName").or_else(|| string(transition, "name")),
            activity_type: string(transition, "activityType").unwrap_or_else(|| "transition".to_string()),
            execution_ids: string(transition, "executionId").into_iter().collect(),
            incident_ids: string_array(transition, "incidentIds"),
            evidence: vec![CurrentActivityEvidence::Transition],
            cancellable: false,
        });
    }
}

// Текущие активности: runtime-дерево, дополненное признаками из инцидентов, job, external task и незакрытой истории.
fn build_current_activities(
    runtime_activities: Vec<ActiveActivityInstance>,
    activity_history: &[ActivityExecutionSummary],
    incidents: &[ProcessIncident],
    jobs: &[ProcessJob],
    external_tasks: &[ProcessExternalTask],
    diagram: &BpmnDiagram,
) -> Vec<ActiveActivityInstance> {
    let runtime_activity_ids: HashSet<String> = runtime_activities.iter().map(|activity| activity.activity_id.clone()).collect();
    let mut evidence_by_activity_id: HashMap<String, BTreeSet<CurrentActivityEvidence>> = HashMap::new();
    let mut add_evidence = |activity_id: Option<&str>, source: CurrentActivityEvidence| {
        if let Some(activity_id) = activity_id.filter(|id| !id.trim().is_empty()) {
            evidence_by_activity_id.entry(activity_id.to_string()).or_default().insert(source);
        }
    };
    for incident in incidents {
        add_evidence(incident_activity_id(incident), CurrentActivityEvidence::Incident);
    }
    for job in jobs {
        add_evidence(job.failed_activity_id.as_deref(), CurrentActivityEvidence::Job);
    }
    for task in external_tasks {
        add_evidence(Some(task.activity_id.as_str()), CurrentActivityEvidence::ExternalTask);
    }
    for summary in activity_history.iter().filter(|summary| summary.active_count > 0 && !runtime_activity_ids.contains(&summary.activity_id)) {
        add_evidence(Some(summary.activity_id.as_str()), CurrentActivityEvidence::UnfinishedHistory);
    }

    let mut activities: Vec<ActiveActivityInstance> = runtime_activities
        .into_iter()
        .map(|mut activity| {
            if let Some(extra) = evidence_by_activity_id.get(&activity.activity_id) {
                let mut evidence: BTreeSet<CurrentActivityEvidence> = activity.evidence.iter().copied().collect();
                evidence.extend(extra.iter().copied());
                activity.evidence = evidence.into_iter().collect();
            }
            activity
        })
        .collect();
    activities.extend(
        evidence_by_activity_id
            .iter()
            .filter(|(activity_id, _)| !runtime_activity_ids.contains(*activity_id))
            .map(|(activity_id, evidence)| {
                let node = diagram.node(activity_id);
                ActiveActivityInstance {
                    id: format!("diagnostic:{activity_id}"),
                    activity_id: activity_id.clone(),
                    activity_name: node.and_then(|node| node.name.clone()),
                    activity_type: node.map(|node| node.node_type.clone()).unwrap_or_else(|| "unknown".to_string()),
                    execution_ids: Vec::new(),
                    incident_ids: incidents
                        .iter()
                        .filter(|incident| incident_activity_id(incident) == Some(activity_id.as_str()))
                        .map(|incident| incident.id.clone())
                        .collect(),
                    evidence: evidence.iter().copied().collect(),
                    cancellable: false,
                }
            }),
    );
    activities.sort_by_cached_key(|activity| (activity_sort_name(activity.activity_name.as_deref(), &activity.activity_id), activity.id.clone()));
    activities
}

fn incident_activity_id(incident: &ProcessIncident) -> Option<&str> {
    incident.failed_activity_id.as_deref().or(incident.activity_id.as_deref())
}

fn activity_sort_name(activity_name: Option<&str>, activity_id: &str) -> String {
    activity_name.map(str::to_lowercase).unwrap_or_else(|| activity_id.to_lowercase())
}

fn parse_historic_activity_instances(body: &str) -> ApiResult<Vec<HistoricActivityInstance>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(HistoricActivityInstance {
                id: string(value, "id")?,
                activity_id: string(value, "activityId")?,
                activity_name: string(value, "activityName"),
                activity_type: string(value, "activityType").unwrap_or_else(|| "unknown".to_string()),
                process_definition_id: string(value, "processDefinitionId"),
                process_instance_id: string(value, "processInstanceId"),
                parent_activity_instance_id: string(value, "parentActivityInstanceId"),
                start_time: string(value, "startTime"),
                end_time: string(value, "endTime"),
                canceled: boolean(value, "canceled"),
            })
        })
        .collect())
}

fn to_activity_timeline(history: &[HistoricActivityInstance]) -> Vec<ActivityTimelineEntry> {
    history
        .iter()
        .filter(|activity| activity.activity_type != "processDefinition")
        .map(|activity| ActivityTimelineEntry {
            id: activity.id.clone(),
            activity_id: activity.activity_id.clone(),
            activity_name: activity.activity_name.clone(),
            activity_type: activity.activity_type.clone(),
            start_time: activity.start_time.clone(),
            end_time: activity.end_time.clone(),
            duration_millis: dates::duration_millis(activity.start_time.as_deref(), activity.end_time.as_deref()),
            canceled: activity.canceled,
        })
        .collect()
}

fn to_activity_execution_summaries(history: &[HistoricActivityInstance]) -> Vec<ActivityExecutionSummary> {
    let grouped = group_in_order(
        history.iter().filter(|activity| activity.activity_type != "processDefinition"),
        |activity| activity.activity_id.clone(),
    );
    let mut summaries: Vec<ActivityExecutionSummary> = grouped
        .into_iter()
        .filter_map(|(activity_id, executions)| {
            let representative = executions.last()?;
            Some(ActivityExecutionSummary {
                activity_id,
                activity_name: executions
                    .iter()
                    .rev()
                    .find_map(|execution| execution.activity_name.as_deref().filter(|name| !name.trim().is_empty()))
                    .map(str::to_string),
                activity_type: representative.activity_type.clone(),
                completed_count: executions.iter().filter(|execution| execution.end_time.is_some() && !execution.canceled).count() as i32,
                active_count: executions.iter().filter(|execution| execution.end_time.is_none()).count() as i32,
                canceled_count: executions.iter().filter(|execution| execution.canceled).count() as i32,
            })
        })
        .collect();
    summaries.sort_by_cached_key(|summary| activity_sort_name(summary.activity_name.as_deref(), &summary.activity_id));
    summaries
}

fn parse_incidents(body: &str) -> ApiResult<Vec<ProcessIncidentWithInstance>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            let id = string(value, "id")?;
            Some(ProcessIncidentWithInstance {
                process_instance_id: string(value, "processInstanceId").unwrap_or_default(),
                incident: ProcessIncident {
                    id,
                    incident_type: string(value, "incidentType").unwrap_or_else(|| "unknown".to_string()),
                    activity_id: string(value, "activityId"),
                    failed_activity_id: string(value, "failedActivityId"),
                    message: string(value, "incidentMessage"),
                    timestamp: string(value, "incidentTimestamp").or_else(|| string(value, "createTime")),
                    configuration: string(value, "configuration"),
                    process_definition_id: string(value, "processDefinitionId"),
                    process_instance_id: string(value, "processInstanceId"),
                    execution_id: string(value, "executionId"),
                    tenant_id: string(value, "tenantId"),
                    annotation: string(value, "annotation"),
                },
            })
        })
        .collect())
}

fn parse_plain_incidents(body: &str) -> ApiResult<Vec<ProcessIncident>> {
    Ok(parse_incidents(body)?.into_iter().map(|incident| incident.incident).collect())
}

fn parse_jobs(body: &str) -> ApiResult<Vec<ProcessJob>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(ProcessJob {
                id: string(value, "id")?,
                retries: int(value, "retries"),
                failed_activity_id: string(value, "failedActivityId"),
                exception_message: string(value, "exceptionMessage"),
                due_date: string(value, "dueDate"),
                suspended: boolean(value, "suspended"),
            })
        })
        .collect())
}

fn parse_external_tasks(body: &str) -> ApiResult<Vec<ProcessExternalTask>> {
    Ok(json_array(body)?
        .iter()
        .filter_map(|value| {
            Some(ProcessExternalTask {
                id: string(value, "id")?,
                activity_id: string(value, "activityId").unwrap_or_default(),
                topic_name: string(value, "topicName").unwrap_or_default(),
                retries: nullable_int(value, "retries"),
                error_message: string(value, "errorMessage"),
                worker_id: string(value, "workerId"),
                lock_expiration_time: string(value, "lockExpirationTime"),
                suspended: boolean(value, "suspended"),
            })
        })
        .collect())
}

fn parse_count(body: &str) -> ApiResult<i32> {
    Ok(int(&json_object(body)?, "count"))
}

fn build_timeline(
    range: &CamundaDateRange,
    started: &[ProcessInstanceSummary],
    completed: &[ProcessInstanceSummary],
    incidents: &[ProcessIncident],
) -> Vec<DashboardTimelinePoint> {
    dates::timeline_buckets(range)
        .into_iter()
        .map(|bucket| DashboardTimelinePoint {
            label: bucket.label,
            started: started.iter().filter(|instance| dates::is_inside(instance.start_time.as_deref(), bucket.start, bucket.end)).count() as i32,
            completed: completed.iter().filter(|instance| dates::is_inside(instance.end_time.as_deref(), bucket.start, bucket.end)).count() as i32,
            incidents: incidents.iter().filter(|incident| dates::is_inside(incident.timestamp.as_deref(), bucket.start, bucket.end)).count() as i32,
        })
        .collect()
}

fn build_duration_buckets(durations: &[i64]) -> Vec<DashboardDurationBucket> {
    let minute = 60_000i64;
    let hour = 60 * minute;
    let count = |from: i64, to: Option<i64>| {
        durations.iter().filter(|&&duration| duration >= from && to.is_none_or(|to| duration < to)).count() as i32
    };
    vec![
        DashboardDurationBucket { label: "< 1 мин".to_string(), count: count(i64::MIN, Some(minute)) },
        DashboardDurationBucket { label: "1–5 мин".to_string(), count: count(minute, Some(5 * minute)) },
        DashboardDurationBucket { label: "5–15 мин".to_string(), count: count(5 * minute, Some(15 * minute)) },
        DashboardDurationBucket { label: "15–60 мин".to_string(), count: count(15 * minute, Some(hour)) },
        DashboardDurationBucket { label: "1–4 ч".to_string(), count: count(hour, Some(4 * hour)) },
        DashboardDurationBucket { label: "4–24 ч".to_string(), count: count(4 * hour, Some(24 * hour)) },
        DashboardDurationBucket { label: "> 24 ч".to_string(), count: count(24 * hour, None) },
    ]
}

fn average(sorted: &[i64]) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    Some((sorted.iter().sum::<i64>() as f64 / sorted.len() as f64) as i64)
}

// Перцентиль с линейной интерполяцией между соседними значениями отсортированного списка.
fn percentile(sorted: &[i64], value: f64) -> Option<i64> {
    let last_index = sorted.len().checked_sub(1)?;
    let position = last_index as f64 * value.clamp(0.0, 1.0);
    let lower_index = position as usize;
    let upper_index = (lower_index + 1).min(last_index);
    let fraction = position - lower_index as f64;
    Some(sorted[lower_index] + ((sorted[upper_index] - sorted[lower_index]) as f64 * fraction) as i64)
}

// Для «Авто» подбирает все правдоподобные типы значения: строку, число и логическое.
fn search_json_values(value: &str, value_type: VariableValueType) -> ApiResult<Vec<Value>> {
    let trimmed = value.trim();
    let as_long = trimmed.parse::<i64>().ok();
    let as_double = trimmed.parse::<f64>().ok().filter(|number| number.is_finite());
    let as_boolean = match trimmed.to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };
    match value_type {
        VariableValueType::Auto => {
            let mut values = vec![Value::from(value)];
            if let Some(number) = as_long {
                values.push(Value::from(number));
            } else if let Some(number) = as_double {
                values.push(Value::from(number));
            }
            if let Some(flag) = as_boolean {
                values.push(Value::from(flag));
            }
            let mut seen = HashSet::new();
            Ok(values.into_iter().filter(|value| seen.insert(value.to_string())).collect())
        }
        VariableValueType::String => Ok(vec![Value::from(value)]),
        VariableValueType::Number => as_long
            .map(Value::from)
            .or_else(|| as_double.map(Value::from))
            .map(|number| vec![number])
            .ok_or_else(|| ApiError::Invalid("Значение переменной должно быть корректным числом.".to_string())),
        VariableValueType::Boolean => as_boolean
            .map(|flag| vec![Value::from(flag)])
            .ok_or_else(|| ApiError::Invalid("Логическое значение должно быть true или false.".to_string())),
    }
}

fn variable_json_value(update: &ProcessVariableUpdate) -> Result<Value, String> {
    let value = update.value.trim();
    match update.value_type.to_lowercase().as_str() {
        "boolean" => match value.to_lowercase().as_str() {
            "true" => Ok(Value::from(true)),
            "false" => Ok(Value::from(false)),
            _ => Err("Для Boolean допустимы только true или false.".to_string()),
        },
        "short" => value.parse::<i16>().map(Value::from).map_err(|_| "Значение должно быть целым числом Short.".to_string()),
        "integer" => value.parse::<i32>().map(Value::from).map_err(|_| "Значение должно быть целым числом Integer.".to_string()),
        "long" => value.parse::<i64>().map(Value::from).map_err(|_| "Значение должно быть целым числом Long.".to_string()),
        "double" => value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(Value::from)
            .ok_or_else(|| "Значение должно быть конечным числом Double.".to_string()),
        "null" => Ok(Value::Null),
        "json" => serde_json::from_str::<Value>(&update.value)
            .map(|_| Value::from(update.value.as_str()))
            .map_err(|_| "Значение должно содержать корректный JSON.".to_string()),
        _ => Ok(Value::from(update.value.as_str())),
    }
}

fn count_in_order<K: Eq + Hash + Clone>(keys: impl IntoIterator<Item = K>) -> Vec<(K, i32)> {
    let mut counts: Vec<(K, i32)> = Vec::new();
    let mut index: HashMap<K, usize> = HashMap::new();
    for key in keys {
        match index.get(&key) {
            Some(&position) => counts[position].1 += 1,
            None => {
                index.insert(key.clone(), counts.len());
                counts.push((key, 1));
            }
        }
    }
    counts
}

fn group_in_order<T, K: Eq + Hash + Clone>(items: impl IntoIterator<Item = T>, key: impl Fn(&T) -> K) -> Vec<(K, Vec<T>)> {
    let mut groups: Vec<(K, Vec<T>)> = Vec::new();
    let mut index: HashMap<K, usize> = HashMap::new();
    for item in items {
        let item_key = key(&item);
        match index.get(&item_key) {
            Some(&position) => groups[position].1.push(item),
            None => {
                index.insert(item_key.clone(), groups.len());
                groups.push((item_key, vec![item]));
            }
        }
    }
    groups
}

fn json_array(body: &str) -> ApiResult<Vec<Value>> {
    match serde_json::from_str::<Value>(body)? {
        Value::Array(values) => Ok(values),
        _ => Err(ApiError::Other("Camunda вернула не список, а другой JSON".to_string())),
    }
}

fn json_object(body: &str) -> ApiResult<Value> {
    let value: Value = serde_json::from_str(body)?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(ApiError::Other("Camunda вернула не объект, а другой JSON".to_string()))
    }
}

fn as_array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn primitive_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn string(value: &Value, name: &str) -> Option<String> {
    value.get(name).and_then(primitive_content)
}

fn int(value: &Value, name: &str) -> i32 {
    nullable_int(value, name).unwrap_or(0)
}

fn nullable_int(value: &Value, name: &str) -> Option<i32> {
    match value.get(name)? {
        Value::Number(number) => number.as_i64().and_then(|number| i32::try_from(number).ok()),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn boolean(value: &Value, name: &str) -> bool {
    nullable_boolean(value, name).unwrap_or(false)
}

fn nullable_boolean(value: &Value, name: &str) -> Option<bool> {
    match value.get(name)? {
        Value::Bool(flag) => Some(*flag),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn string_array(value: &Value, name: &str) -> Vec<String> {
    as_array(value.get(name)).iter().filter_map(primitive_content).collect()
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{any, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use super::*;
    use crate::camunda::catalog::InMemoryProcessVariableCatalogRepository;

    fn connection(server: &MockServer) -> CamundaConnection {
        CamundaConnection {
            id: "test".to_string(),
            name: "Test".to_string(),
            rest_url: format!("{}/engine-rest", server.uri()),
            cockpit_url: "http://127.0.0.1/camunda/app/cockpit".to_string(),
            environment: Environment::Test,
            engine_version: None,
        }
    }

    fn api() -> CamundaApi {
        CamundaApi::new(http::camunda_client(), Arc::new(InMemoryProcessVariableCatalogRepository::default()))
    }

    fn api_at(now: &str, repository: Arc<dyn ProcessVariableCatalogRepository>) -> CamundaApi {
        let fixed = DateTime::parse_from_rfc3339(now).expect("дата теста").with_timezone(&Utc);
        CamundaApi::new(http::camunda_client(), repository).with_clock(move || fixed)
    }

    fn custom(from: &str, to: &str) -> DashboardDateFilter {
        DashboardDateFilter { preset: DashboardDatePreset::Custom, from_date: from.to_string(), to_date: to.to_string() }
    }

    fn json_response(status: u16, body: &str) -> ResponseTemplate {
        ResponseTemplate::new(status).set_body_raw(body.to_string(), "application/json")
    }

    async fn mount(server: &MockServer, http_method: &str, route: &str, status: u16, body: &str) {
        Mock::given(method(http_method)).and(path(route)).respond_with(json_response(status, body)).mount(server).await;
    }

    async fn requests_to(server: &MockServer, route: &str) -> Vec<Request> {
        server.received_requests().await.unwrap_or_default().into_iter().filter(|request| request.url.path() == route).collect()
    }

    fn query(request: &Request) -> String {
        request.url.query().unwrap_or_default().to_string()
    }

    fn body_json(request: &Request) -> Value {
        serde_json::from_slice(&request.body).expect("JSON тела запроса")
    }

    fn has_completed_flag(request: &Request) -> bool {
        query(request).contains("completed=true")
    }

    fn sum_instances(dashboard: &ProcessDashboard) -> i32 {
        dashboard.definitions.iter().map(|definition| definition.instances).sum()
    }

    fn sum_incidents(dashboard: &ProcessDashboard) -> i32 {
        dashboard.definitions.iter().map(|definition| definition.incidents).sum()
    }

    #[tokio::test]
    async fn dashboard_joins_latest_definitions_with_runtime_statistics() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[
                {"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null},
                {"id":"beta:4:def","key":"beta","name":null,"version":4,"deploymentId":"dep-4","suspended":true,"tenantId":"b2c"}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/statistics",
            200,
            r#"[
                {"id":"alpha:2:abc","instances":6,"incidents":[{"incidentType":"failedJob","incidentCount":2}]},
                {"id":"beta:4:def","instances":24,"incidents":[{"incidentType":"failedJob","incidentCount":3},{"incidentType":"externalTask","incidentCount":1}]}
            ]"#,
        )
        .await;

        let dashboard = api().load_dashboard(&connection(&server), &DashboardDateFilter::default()).await.expect("дашборд");

        assert_eq!(dashboard.definitions.len(), 2);
        assert_eq!(sum_instances(&dashboard), 30);
        assert_eq!(sum_incidents(&dashboard), 6);
        assert_eq!(dashboard.definitions[1].display_name(), "beta");
        assert!(dashboard.definitions[1].suspended);
        assert_eq!(dashboard.timeline_title, "Динамика за последние 24 часа");
        assert_eq!(dashboard.completed_instances, 0);
        let definition_query = query(&requests_to(&server, "/engine-rest/process-definition").await[0]);
        assert!(definition_query.contains("latestVersion=true"), "{definition_query}");
        assert!(definition_query.contains("sortBy=deployTime"), "{definition_query}");
        assert!(definition_query.contains("sortOrder=desc"), "{definition_query}");
    }

    #[tokio::test]
    async fn variable_search_sends_typed_post_body_and_parses_instances() {
        let server = MockServer::start().await;
        mount(
            &server,
            "POST",
            "/engine-rest/process-instance",
            200,
            r#"[{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","businessKey":"BR-42","ended":false,"suspended":false,"tenantId":"b2c"}]"#,
        )
        .await;

        let instances = api()
            .search_process_instances(&connection(&server), "loan", "applicationId", "42", VariableValueType::Number)
            .await
            .expect("поиск");

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].business_key.as_deref(), Some("BR-42"));
        let requests = requests_to(&server, "/engine-rest/process-instance").await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "POST");
        assert!(query(&requests[0]).contains("maxResults=100"));
        let json = body_json(&requests[0]);
        assert_eq!(json["processDefinitionKey"], "loan");
        let variables = json["variables"].as_array().expect("массив переменных");
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0]["name"], "applicationId");
        assert_eq!(variables[0]["operator"], "eq");
        assert_eq!(variables[0]["value"], 42);
        assert!(variables[0]["value"].is_number());
        assert_eq!(json["sorting"][0]["sortBy"], "instanceId");
    }

    #[tokio::test]
    async fn automatic_variable_search_tries_string_and_detected_primitive_type() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/engine-rest/process-instance"))
            .and(|request: &Request| {
                serde_json::from_slice::<Value>(&request.body)
                    .map(|body| body["variables"][0]["value"].is_string())
                    .unwrap_or(false)
            })
            .respond_with(json_response(
                200,
                r#"[{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false}]"#,
            ))
            .mount(&server)
            .await;
        mount(&server, "POST", "/engine-rest/process-instance", 200, "[]").await;

        let instances = api()
            .search_process_instances(&connection(&server), "loan", "applicationId", "42", VariableValueType::Auto)
            .await
            .expect("поиск");

        assert_eq!(requests_to(&server, "/engine-rest/process-instance").await.len(), 2);
        assert_eq!(instances.iter().map(|instance| instance.id.as_str()).collect::<Vec<_>>(), vec!["instance-1"]);
    }

    #[tokio::test]
    async fn invalid_typed_search_value_fails_before_request() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(json_response(200, "[]")).mount(&server).await;

        let number = api().search_process_instances(&connection(&server), "loan", "amount", "abc", VariableValueType::Number).await;
        let boolean = api().search_process_instances(&connection(&server), "loan", "flag", "yes", VariableValueType::Boolean).await;

        assert!(number.expect_err("число").starts_with("Значение переменной должно быть корректным числом.\nЗапрос: "));
        assert!(boolean.expect_err("boolean").starts_with("Логическое значение должно быть true или false.\nЗапрос: "));
        assert!(server.received_requests().await.unwrap_or_default().is_empty());
    }

    #[tokio::test]
    async fn search_catalog_loads_processes_and_concrete_variables_from_their_active_instances() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[
                {"id":"loan:3:def","key":"loan","name":"Кредит","version":3,"deploymentId":"dep","suspended":false},
                {"id":"pledge:1:def","key":"pledge","name":"Залог","version":1,"deploymentId":"dep","suspended":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance",
            200,
            r#"[
                {"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false},
                {"id":"instance-2","definitionId":"loan:3:def","definitionKey":"loan","ended":false,"suspended":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/variable-instance",
            200,
            r#"[
                {"id":"v1","name":"applicationId","type":"String","processInstanceId":"instance-1"},
                {"id":"v2","name":"applicationId","type":"String","processInstanceId":"instance-2"},
                {"id":"v3","name":"amount","type":"Long","processInstanceId":"instance-2"}
            ]"#,
        )
        .await;
        let connection = connection(&server);
        let repository: Arc<dyn ProcessVariableCatalogRepository> = Arc::new(InMemoryProcessVariableCatalogRepository::default());
        let api = api_at("2026-07-24T00:00:00Z", repository.clone());

        let definitions = api.load_search_process_definitions(&connection).await.expect("процессы");
        let catalog = api.load_process_variable_catalog(&connection, "loan").await.expect("каталог");

        assert_eq!(definitions.iter().map(|definition| definition.display_name()).collect::<Vec<_>>(), vec!["Залог", "Кредит"]);
        assert!(query(&requests_to(&server, "/engine-rest/process-definition").await[0]).contains("latestVersion=true"));
        let instance_query = query(&requests_to(&server, "/engine-rest/process-instance").await[0]);
        assert!(instance_query.contains("processDefinitionKey=loan"), "{instance_query}");
        assert!(instance_query.contains("maxResults=101"), "{instance_query}");
        let variable_query = query(&requests_to(&server, "/engine-rest/variable-instance").await[0]);
        assert!(variable_query.contains("processInstanceIdIn=instance-1%2Cinstance-2"), "{variable_query}");
        assert!(variable_query.contains("deserializeValues=false"), "{variable_query}");
        assert_eq!(catalog.process_definition_key, "loan");
        assert_eq!(catalog.inspected_instance_count, 2);
        assert!(!catalog.instances_truncated);
        assert_eq!(catalog.variables.iter().map(|variable| variable.name.as_str()).collect::<Vec<_>>(), vec!["amount", "applicationId"]);
        assert_eq!(catalog.variables[1].occurrences, 2);
        assert_eq!(catalog.variables[0].types, vec!["Long".to_string()]);

        api.load_process_variable_catalog(&connection, "loan").await.expect("каталог из кэша");
        assert_eq!(requests_to(&server, "/engine-rest/variable-instance").await.len(), 1, "Повторное открытие процесса должно использовать кэш каталога");

        api_at("2026-07-24T01:00:00Z", repository.clone()).load_process_variable_catalog(&connection, "loan").await.expect("каталог с диска");
        assert_eq!(requests_to(&server, "/engine-rest/variable-instance").await.len(), 1, "Новый сеанс приложения должен использовать дисковый кэш каталога");

        api_at("2026-07-25T00:00:01Z", repository.clone()).load_process_variable_catalog(&connection, "loan").await.expect("каталог после TTL");
        assert_eq!(requests_to(&server, "/engine-rest/variable-instance").await.len(), 2, "Просроченный кэш перечитывается");

        api.invalidate_process_variable_catalog(&connection, "loan");
        assert!(repository.load(&connection.rest_url, "loan").is_none());
        api.load_process_variable_catalog(&connection, " loan ").await.expect("каталог после сброса");
        assert_eq!(requests_to(&server, "/engine-rest/variable-instance").await.len(), 3);
    }

    #[tokio::test]
    async fn forbidden_dashboard_returns_specific_failure() {
        let server = MockServer::start().await;
        mount(&server, "GET", "/engine-rest/process-definition", 403, "Forbidden").await;

        let message = api().load_dashboard(&connection(&server), &DashboardDateFilter::default()).await.expect_err("ошибка");

        assert!(message.contains("Доступ запрещён (HTTP 403)"), "{message}");
        assert!(message.contains("\nЗапрос: http://"), "{message}");
        assert!(message.contains("/engine-rest/process-definition?latestVersion=true"), "{message}");
    }

    #[tokio::test]
    async fn custom_period_uses_historic_count_endpoints() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}]"#,
        )
        .await;
        mount(&server, "GET", "/engine-rest/history/process-instance/count", 200, r#"{"count":17}"#).await;
        mount(&server, "GET", "/engine-rest/history/incident/count", 200, r#"{"count":3}"#).await;

        let dashboard = api().load_dashboard(&connection(&server), &custom("2026-07-15", "2026-07-16")).await.expect("дашборд");

        assert_eq!(sum_instances(&dashboard), 17);
        assert_eq!(sum_incidents(&dashboard), 3);
        assert_eq!(dashboard.timeline_title, "Динамика за период 2026-07-15 — 2026-07-16");
        let process_queries: Vec<String> =
            requests_to(&server, "/engine-rest/history/process-instance/count").await.iter().map(query).collect();
        let definition_query = process_queries.iter().find(|query| query.contains("processDefinitionId=")).expect("запрос по определению");
        assert!(definition_query.contains("processDefinitionId=alpha%3A2%3Aabc"), "{definition_query}");
        assert!(definition_query.contains("startedAfter="), "{definition_query}");
        assert!(definition_query.contains("startedBefore="), "{definition_query}");
        let incident_query = query(&requests_to(&server, "/engine-rest/history/incident/count").await[0]);
        assert!(incident_query.contains("createTimeAfter="), "{incident_query}");
        assert!(incident_query.contains("createTimeBefore="), "{incident_query}");
    }

    #[tokio::test]
    async fn dashboard_builds_started_completed_and_incident_timeline() {
        let server = MockServer::start().await;
        let bpmn_xml = r#"<?xml version="1.0" encoding="UTF-8"?><definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"><process id="alpha"><serviceTask id="sendDocuments" name="Отправить документы" /></process><bpmndi:BPMNDiagram><bpmndi:BPMNPlane bpmnElement="alpha"><bpmndi:BPMNShape bpmnElement="sendDocuments"><dc:Bounds x="100" y="100" width="100" height="80" /></bpmndi:BPMNShape></bpmndi:BPMNPlane></bpmndi:BPMNDiagram></definitions>"#;
        mount(&server, "GET", "/engine-rest/process-definition/alpha%3A2%3Aabc/xml", 200, &json!({ "bpmn20Xml": bpmn_xml }).to_string()).await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}]"#,
        )
        .await;
        Mock::given(method("GET"))
            .and(path("/engine-rest/history/process-instance/count"))
            .and(has_completed_flag)
            .respond_with(json_response(200, r#"{"count":2}"#))
            .mount(&server)
            .await;
        mount(&server, "GET", "/engine-rest/history/process-instance/count", 200, r#"{"count":3}"#).await;
        mount(&server, "GET", "/engine-rest/history/incident/count", 200, r#"{"count":2}"#).await;
        Mock::given(method("GET"))
            .and(path("/engine-rest/history/process-instance"))
            .and(has_completed_flag)
            .respond_with(json_response(
                200,
                r#"[
                    {"id":"done-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T08:00:00.000+0800","endTime":"2026-07-15T08:30:00.000+0800","state":"COMPLETED"},
                    {"id":"done-2","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-16T08:00:00.000+0800","endTime":"2026-07-16T10:00:00.000+0800","state":"COMPLETED"}
                ]"#,
            ))
            .mount(&server)
            .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[
                {"id":"started-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T08:00:00.000+0800"},
                {"id":"started-2","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-15T11:00:00.000+0800"},
                {"id":"started-3","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","startTime":"2026-07-16T08:00:00.000+0800"}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/incident",
            200,
            r#"[
                {"id":"incident-1","incidentType":"failedJob","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:abc","processInstanceId":"started-1","createTime":"2026-07-15T09:00:00.000+0800"},
                {"id":"incident-2","incidentType":"failedExternalTask","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:abc","processInstanceId":"started-3","createTime":"2026-07-16T09:00:00.000+0800"}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/activity-instance",
            200,
            r#"[
                {"id":"activity-ancient","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"ancient-300-days","startTime":"2025-09-19T08:00:00.000+0800"},
                {"id":"activity-old","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"started-1","startTime":"2026-07-15T08:00:00.000+0800"},
                {"id":"activity-new","activityId":"sendDocuments","activityName":"Отправить документы","activityType":"serviceTask","processDefinitionId":"alpha:2:abc","processInstanceId":"started-3","startTime":"2026-07-16T08:00:00.000+0800"}
            ]"#,
        )
        .await;
        let repository: Arc<dyn ProcessVariableCatalogRepository> = Arc::new(InMemoryProcessVariableCatalogRepository::default());

        let api = api_at("2026-07-16T12:00:00Z", repository);

        let dashboard = api.load_dashboard(&connection(&server), &custom("2026-07-15", "2026-07-16")).await.expect("дашборд");

        assert_eq!(sum_instances(&dashboard), 3);
        assert_eq!(dashboard.completed_instances, 2);
        assert_eq!(sum_incidents(&dashboard), 2);
        assert_eq!(dashboard.timeline.len(), 48);
        assert_eq!(dashboard.timeline.iter().map(|point| point.started).sum::<i32>(), 3);
        assert_eq!(dashboard.timeline.iter().map(|point| point.completed).sum::<i32>(), 2);
        assert_eq!(dashboard.timeline.iter().map(|point| point.incidents).sum::<i32>(), 2);
        assert_eq!(dashboard.average_duration_millis, Some(4_500_000));
        assert_eq!(dashboard.median_duration_millis, Some(4_500_000));
        assert_eq!(dashboard.p95_duration_millis, Some(6_930_000));
        let bucket = |label: &str| dashboard.duration_buckets.iter().find(|bucket| bucket.label == label).map(|bucket| bucket.count);
        assert_eq!(bucket("15–60 мин"), Some(1));
        assert_eq!(bucket("1–4 ч"), Some(1));
        assert_eq!(bucket("< 1 мин"), Some(0));
        let incident_types: HashSet<&str> = dashboard.incident_types.iter().map(|value| value.incident_type.as_str()).collect();
        assert_eq!(incident_types, HashSet::from(["failedJob", "failedExternalTask"]));
        assert_eq!(dashboard.incident_activities.len(), 1);
        assert_eq!(dashboard.incident_activities[0].activity_id, "sendDocuments");
        assert_eq!(dashboard.incident_activities[0].activity_name.as_deref(), Some("Отправить документы"));
        assert_eq!(dashboard.incident_activities[0].count, 2);
        assert!(query(&requests_to(&server, "/engine-rest/history/activity-instance").await[0]).contains("unfinished=true"));
        assert_eq!(dashboard.longest_waiting_activities.len(), 1);
        let waiting = &dashboard.longest_waiting_activities[0];
        assert_eq!(waiting.activity_name.as_deref(), Some("Отправить документы"));
        assert_eq!(waiting.instance_count, 2);
        assert_eq!(waiting.oldest_process_instance_id, "started-1");
        assert_eq!(waiting.maximum_waiting_millis, 129_600_000);
        assert_eq!(waiting.average_waiting_millis, 86_400_000);
        assert_eq!(waiting.oldest_started_at.as_deref(), Some("2026-07-15T08:00:00.000+0800"));
        assert!(!dashboard.timeline_truncated);
        let xml_route = "/engine-rest/process-definition/alpha%3A2%3Aabc/xml";
        let xml_requests = requests_to(&server, xml_route).await.len();
        assert!((1..=2).contains(&xml_requests), "{xml_requests}");
        api.load_dashboard(&connection(&server), &custom("2026-07-15", "2026-07-16")).await.expect("повторный дашборд");
        assert_eq!(requests_to(&server, xml_route).await.len(), xml_requests, "BPMN берётся из кэша при повторной загрузке");
    }

    #[tokio::test]
    async fn incidents_can_be_filtered_by_day_and_all_versions_of_process() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition",
            200,
            r#"[
                {"id":"alpha:3:new","key":"alpha","name":"Alpha","version":3,"versionTag":"release-3","deploymentId":"dep-3","suspended":false},
                {"id":"alpha:2:old","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/incident",
            200,
            r#"[{"id":"incident-1","incidentType":"failedJob","failedActivityId":"sendDocuments","processDefinitionId":"alpha:2:old","createTime":"2026-07-15T09:00:00.000+0800"}]"#,
        )
        .await;
        let connection = connection(&server);

        let versions = api().load_process_definition_versions(&connection, "alpha").await.expect("версии");
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version_tag.as_deref(), Some("release-3"));
        let definition_query = query(&requests_to(&server, "/engine-rest/process-definition").await[0]);
        assert!(definition_query.contains("key=alpha"), "{definition_query}");
        assert!(definition_query.contains("sortBy=version"), "{definition_query}");

        let incidents = api()
            .load_incidents(&connection, None, &custom("2026-07-15", "2026-07-15"), Some("alpha"))
            .await
            .expect("инциденты");
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].timestamp.as_deref(), Some("2026-07-15T09:00:00.000+0800"));
        let incident_query = query(&requests_to(&server, "/engine-rest/history/incident").await[0]);
        assert!(incident_query.contains("processDefinitionKeyIn=alpha"), "{incident_query}");
        assert!(incident_query.contains("createTimeAfter="), "{incident_query}");
        assert!(incident_query.contains("createTimeBefore="), "{incident_query}");
    }

    #[tokio::test]
    async fn live_incidents_use_runtime_endpoint_sorted_newest_first() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/incident",
            200,
            r#"[
                {"id":"no-time","incidentType":"failedJob"},
                {"id":"old","incidentType":"failedJob","incidentTimestamp":"2026-07-15T09:00:00.000+0800"},
                {"id":"new","incidentType":"failedExternalTask","incidentTimestamp":"2026-07-16T09:00:00.000+0800"}
            ]"#,
        )
        .await;

        let incidents = api()
            .load_incidents(&connection(&server), Some("alpha:2:abc"), &DashboardDateFilter::default(), None)
            .await
            .expect("инциденты");

        assert_eq!(incidents.iter().map(|incident| incident.id.as_str()).collect::<Vec<_>>(), vec!["new", "old", "no-time"]);
        let incident_query = query(&requests_to(&server, "/engine-rest/incident").await[0]);
        assert!(incident_query.starts_with("processDefinitionId=alpha%3A2%3Aabc&firstResult=0&maxResults=1000"), "{incident_query}");
    }

    #[tokio::test]
    async fn teleport_uses_activity_instance_id_and_start_target() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;

        api()
            .teleport_process_instance(
                &connection(&server),
                "instance-1",
                &TeleportRequest {
                    source_activity_instance_id: "activity-instance-7".to_string(),
                    target_activity_id: "Activity_target".to_string(),
                    annotation: "INC-42".to_string(),
                    skip_custom_listeners: true,
                    skip_io_mappings: true,
                },
            )
            .await
            .expect("телепорт");

        let requests = server.received_requests().await.unwrap_or_default();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "POST");
        assert_eq!(requests[0].url.path(), "/engine-rest/process-instance/instance-1/modification");
        let payload = body_json(&requests[0]);
        assert_eq!(payload["annotation"], "INC-42");
        assert_eq!(payload["skipCustomListeners"], true);
        assert_eq!(payload["skipIoMappings"], true);
        let instructions = payload["instructions"].as_array().expect("инструкции");
        assert_eq!(instructions[0]["type"], "cancel");
        assert_eq!(instructions[0]["activityInstanceId"], "activity-instance-7");
        assert_eq!(instructions[1]["type"], "startBeforeActivity");
        assert_eq!(instructions[1]["activityId"], "Activity_target");
    }

    #[tokio::test]
    async fn recovery_operations_use_expected_endpoints_and_bodies() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;
        let connection = connection(&server);
        let api = api();

        api.set_job_retries(&connection, "job-1", 3).await.expect("retries job");
        api.set_external_task_retries(&connection, "task-1", 3).await.expect("retries external task");
        api.unlock_external_task(&connection, "task-1").await.expect("unlock");
        api.set_process_instance_suspended(&connection, "instance-1", true).await.expect("suspend");

        let requests = server.received_requests().await.unwrap_or_default();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0].method.as_str(), "PUT");
        assert_eq!(requests[0].url.path(), "/engine-rest/job/job-1/retries");
        assert_eq!(body_json(&requests[0])["retries"], 3);
        assert_eq!(requests[1].method.as_str(), "PUT");
        assert_eq!(requests[1].url.path(), "/engine-rest/external-task/task-1/retries");
        assert_eq!(body_json(&requests[1])["retries"], 3);
        assert_eq!(requests[2].method.as_str(), "POST");
        assert_eq!(requests[2].url.path(), "/engine-rest/external-task/task-1/unlock");
        assert!(requests[2].body.is_empty());
        assert_eq!(requests[3].method.as_str(), "PUT");
        assert_eq!(requests[3].url.path(), "/engine-rest/process-instance/instance-1/suspended");
        assert_eq!(body_json(&requests[3])["suspended"], true);
    }

    #[tokio::test]
    async fn variable_update_preserves_type_and_value_info() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;

        api()
            .update_process_variable(
                &connection(&server),
                "instance-1",
                &ProcessVariableUpdate {
                    name: "approved by support".to_string(),
                    value_type: "Boolean".to_string(),
                    value: "true".to_string(),
                    value_info: Some("{}".to_string()),
                },
            )
            .await
            .expect("обновление переменной");

        let requests = server.received_requests().await.unwrap_or_default();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method.as_str(), "PUT");
        assert_eq!(requests[0].url.path(), "/engine-rest/process-instance/instance-1/variables/approved+by+support");
        let payload = body_json(&requests[0]);
        assert_eq!(payload["type"], "Boolean");
        assert_eq!(payload["value"], true);
        assert!(payload["value"].is_boolean());
        assert_eq!(payload["valueInfo"], json!({}));
    }

    #[tokio::test]
    async fn invalid_json_variable_is_rejected_before_request() {
        let server = MockServer::start().await;
        Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;
        let connection = connection(&server);
        let update = |value_type: &str, value: &str| ProcessVariableUpdate {
            name: "payload".to_string(),
            value_type: value_type.to_string(),
            value: value.to_string(),
            value_info: Some("{}".to_string()),
        };

        let json_failure = api().update_process_variable(&connection, "instance-1", &update("Json", "{broken")).await.expect_err("JSON");
        let long_failure = api().update_process_variable(&connection, "instance-1", &update("Long", "12.5")).await.expect_err("Long");
        let boolean_failure = api().update_process_variable(&connection, "instance-1", &update("Boolean", "да")).await.expect_err("Boolean");

        assert!(json_failure.contains("корректный JSON"), "{json_failure}");
        assert_eq!(long_failure, "Значение должно быть целым числом Long.");
        assert_eq!(boolean_failure, "Для Boolean допустимы только true или false.");
        assert!(server.received_requests().await.unwrap_or_default().is_empty());
    }

    #[tokio::test]
    async fn instance_details_combine_runtime_state_variables_failures_and_bpmn() {
        let server = MockServer::start().await;
        let bpmn_xml = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
    xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
    xmlns:dc="http://www.omg.org/spec/DD/20100524/DC">
  <process id="loan">
    <serviceTask id="Task_1" name="Проверка" />
    <serviceTask id="Task_2" name="Ожидание перехода" />
    <userTask id="Task_4" name="Ожидание клиента" />
  </process>
  <bpmndi:BPMNDiagram id="d"><bpmndi:BPMNPlane id="p" bpmnElement="loan">
    <bpmndi:BPMNShape id="s" bpmnElement="Task_1"><dc:Bounds x="10" y="20" width="100" height="80" /></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="s2" bpmnElement="Task_2"><dc:Bounds x="150" y="20" width="100" height="80" /></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="s4" bpmnElement="Task_4"><dc:Bounds x="290" y="20" width="100" height="80" /></bpmndi:BPMNShape>
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance/instance-1",
            200,
            r#"{"id":"instance-1","definitionId":"loan:3:def","definitionKey":"loan","businessKey":"APP-42","ended":false,"suspended":false,"tenantId":"b2c"}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance/instance-1/variables",
            200,
            r#"{"applicationId":{"type":"String","value":"APP-42","valueInfo":{}}}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance/instance-1/activity-instances",
            200,
            r#"{"id":"root","activityId":"loan","activityType":"processDefinition","childActivityInstances":[{"id":"activity-instance-1","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","executionIds":["exec-1"],"incidentIds":["incident-1"]}],"childTransitionInstances":[{"id":"transition-1","activityId":"Task_2","activityName":"Ожидание перехода","activityType":"serviceTask","executionId":"exec-2","incidentIds":[]}]}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/activity-instance",
            200,
            r#"[
                {"id":"historic-1","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:00:00.000+0000","endTime":"2026-07-16T09:00:01.000+0000","canceled":false},
                {"id":"historic-2","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:01:00.000+0000","endTime":"2026-07-16T09:01:01.000+0000","canceled":false},
                {"id":"historic-3","activityId":"Task_1","activityName":"Проверка","activityType":"serviceTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:02:00.000+0000","endTime":null,"canceled":false},
                {"id":"historic-4","activityId":"Task_4","activityName":"Ожидание клиента","activityType":"userTask","processInstanceId":"instance-1","startTime":"2026-07-16T09:03:00.000+0000","endTime":null,"canceled":false}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/incident",
            200,
            r#"[{"id":"incident-1","processInstanceId":"instance-1","incidentType":"failedJob","failedActivityId":"Task_1","incidentMessage":"boom"}]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/job",
            200,
            r#"[{"id":"job-1","retries":0,"failedActivityId":"Task_1","exceptionMessage":"boom","suspended":false}]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/external-task",
            200,
            r#"[{"id":"external-1","activityId":"Task_3","topicName":"loan-check","retries":0,"errorMessage":"worker failed","workerId":"worker-7","lockExpirationTime":"2026-07-16T10:00:00.000+0000","suspended":false}]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/loan%3A3%3Adef/xml",
            200,
            &json!({ "id": "loan:3:def", "bpmn20Xml": bpmn_xml }).to_string(),
        )
        .await;

        let details = api().load_process_instance_details(&connection(&server), "instance-1").await.expect("детали заявки");

        let activity = |activity_id: &str| {
            details.active_activities.iter().find(|activity| activity.activity_id == activity_id).expect("активность")
        };
        assert_eq!(details.instance.business_key.as_deref(), Some("APP-42"));
        assert_eq!(details.variables.len(), 1);
        assert_eq!(details.variables[0].value, "APP-42");
        assert_eq!(details.variables[0].value_type, "String");
        assert_eq!(details.variables[0].value_info, None);
        assert_eq!(details.active_activities.len(), 4);
        assert_eq!(activity("Task_1").id, "activity-instance-1");
        assert_eq!(
            activity("Task_1").evidence,
            vec![CurrentActivityEvidence::RuntimeActivity, CurrentActivityEvidence::Incident, CurrentActivityEvidence::Job]
        );
        assert!(activity("Task_1").cancellable);
        assert_eq!(activity("Task_1").incident_ids, vec!["incident-1".to_string()]);
        assert_eq!(activity("Task_2").evidence, vec![CurrentActivityEvidence::Transition]);
        assert_eq!(activity("Task_2").execution_ids, vec!["exec-2".to_string()]);
        assert_eq!(activity("Task_3").evidence, vec![CurrentActivityEvidence::ExternalTask]);
        assert_eq!(activity("Task_3").id, "diagnostic:Task_3");
        assert_eq!(activity("Task_3").activity_type, "unknown");
        assert!(!activity("Task_3").cancellable);
        assert_eq!(activity("Task_4").evidence, vec![CurrentActivityEvidence::UnfinishedHistory]);
        assert_eq!(activity("Task_4").activity_name.as_deref(), Some("Ожидание клиента"));
        assert_eq!(activity("Task_4").activity_type, "userTask");
        assert!(!activity("Task_4").cancellable);
        let task_1_history = details.activity_history.iter().find(|summary| summary.activity_id == "Task_1").expect("история Task_1");
        assert_eq!(task_1_history.completed_count, 2);
        assert_eq!(task_1_history.active_count, 1);
        assert_eq!(task_1_history.canceled_count, 0);
        assert_eq!(details.incidents.len(), 1);
        assert_eq!(details.incidents[0].message.as_deref(), Some("boom"));
        assert_eq!(details.jobs.len(), 1);
        assert_eq!(details.jobs[0].retries, 0);
        assert_eq!(details.external_tasks.len(), 1);
        assert_eq!(details.external_tasks[0].worker_id.as_deref(), Some("worker-7"));
        let node_ids: HashSet<&str> = details.diagram.nodes.iter().map(|node| node.id.as_str()).collect();
        assert_eq!(node_ids, HashSet::from(["Task_1", "Task_2", "Task_4"]));
        assert_eq!(details.bpmn_xml, bpmn_xml);
        let tenant = details.metadata.iter().find(|field| field.label == "ID тенанта").expect("метаданные");
        assert_eq!(tenant.value, "b2c");
        let case = details.metadata.iter().find(|field| field.label == "ID экземпляра кейса").expect("метаданные");
        assert_eq!(case.value, "—");
        let history_query = query(&requests_to(&server, "/engine-rest/history/activity-instance").await[0]);
        assert!(history_query.contains("processInstanceId=instance-1"), "{history_query}");
        assert!(query(&requests_to(&server, "/engine-rest/process-instance/instance-1/variables").await[0]).contains("deserializeValues=false"));
    }

    #[tokio::test]
    async fn definition_period_loads_historic_instances_from_selected_dates() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/alpha%3A2%3Aabc",
            200,
            r#"{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false,"tenantId":null}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[{"id":"historic-1","processDefinitionId":"alpha:2:abc","processDefinitionKey":"alpha","businessKey":"APP-YESTERDAY","startTime":"2026-07-15T10:00:00.000+0000","endTime":"2026-07-15T10:02:00.000+0000","state":"COMPLETED","tenantId":null}]"#,
        )
        .await;
        mount(&server, "GET", "/engine-rest/history/incident", 200, "[]").await;
        let filter = DashboardDateFilter { preset: DashboardDatePreset::Yesterday, ..DashboardDateFilter::default() };

        let details = api().load_process_definition_details(&connection(&server), "alpha:2:abc", &filter).await.expect("детали процесса");

        assert_eq!(details.instances.len(), 1);
        assert_eq!(details.instances[0].instance.business_key.as_deref(), Some("APP-YESTERDAY"));
        assert!(details.instances[0].instance.ended);
        assert_eq!(details.instances[0].incident_count, 0);
        assert_eq!(details.definition.instances, 1);
        assert_eq!(details.definition.incidents, 0);
        assert_eq!(details.metadata[0], ProcessMetadataField { label: "ID определения".to_string(), value: "alpha:2:abc".to_string() });
        let instance_query = query(&requests_to(&server, "/engine-rest/history/process-instance").await[0]);
        assert!(instance_query.contains("startedAfter="), "{instance_query}");
        assert!(instance_query.contains("startedBefore="), "{instance_query}");
        let incident_query = query(&requests_to(&server, "/engine-rest/history/incident").await[0]);
        assert!(incident_query.contains("sortBy=createTime"), "{incident_query}");
        assert_eq!(details.date_filter.preset, DashboardDatePreset::Yesterday);
    }

    #[tokio::test]
    async fn live_definition_details_count_runtime_incidents_per_instance() {
        let server = MockServer::start().await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-definition/alpha%3A2%3Aabc",
            200,
            r#"{"id":"alpha:2:abc","key":"alpha","name":"Alpha","version":2,"deploymentId":"dep-2","suspended":false}"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/process-instance",
            200,
            r#"[
                {"id":"i-1","definitionId":"alpha:2:abc","definitionKey":"alpha","ended":false,"suspended":false},
                {"id":"i-2","definitionId":"alpha:2:abc","definitionKey":"alpha","ended":false,"suspended":true}
            ]"#,
        )
        .await;
        mount(
            &server,
            "GET",
            "/engine-rest/incident",
            200,
            r#"[
                {"id":"inc-1","processInstanceId":"i-1","incidentType":"failedJob"},
                {"id":"inc-2","processInstanceId":"i-1","incidentType":"failedJob"}
            ]"#,
        )
        .await;

        let details = api()
            .load_process_definition_details(&connection(&server), "alpha:2:abc", &DashboardDateFilter::default())
            .await
            .expect("детали процесса");

        assert_eq!(details.definition.instances, 2);
        assert_eq!(details.definition.incidents, 2);
        assert_eq!(details.instances.iter().map(|item| item.incident_count).collect::<Vec<_>>(), vec![2, 0]);
        let instance_query = query(&requests_to(&server, "/engine-rest/process-instance").await[0]);
        assert!(instance_query.contains("processDefinitionId=alpha%3A2%3Aabc"), "{instance_query}");
        assert!(instance_query.contains("maxResults=500"), "{instance_query}");
    }

    #[test]
    fn percentiles_and_buckets_follow_kotlin_rules() {
        let durations = vec![1_800_000, 7_200_000];
        assert_eq!(percentile(&durations, 0.50), Some(4_500_000));
        assert_eq!(percentile(&durations, 0.95), Some(6_930_000));
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(average(&durations), Some(4_500_000));
        let buckets = build_duration_buckets(&[30_000, 60_000, 299_999, 86_400_000]);
        assert_eq!(buckets.iter().map(|bucket| bucket.count).collect::<Vec<_>>(), vec![1, 2, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn automatic_search_values_cover_every_plausible_type() {
        let render = |value: &str| {
            search_json_values(value, VariableValueType::Auto).expect("значения").iter().map(Value::to_string).collect::<Vec<_>>()
        };
        assert_eq!(render("42"), vec!["\"42\"", "42"]);
        assert_eq!(render("4.5"), vec!["\"4.5\"", "4.5"]);
        assert_eq!(render(" True "), vec!["\" True \"", "true"]);
        assert_eq!(render("APP-1"), vec!["\"APP-1\""]);
    }

    #[tokio::test]
    async fn business_key_search_merges_runtime_history_and_exact_id() {
        let server = MockServer::start().await;
        mount(&server, "GET", "/engine-rest/process-instance", 200, r#"[{"id":"run-1","definitionId":"alpha:1:x","businessKey":"APP-10","suspended":false,"tenantId":null,"ended":false}]"#).await;
        mount(
            &server,
            "GET",
            "/engine-rest/history/process-instance",
            200,
            r#"[{"id":"run-1","processDefinitionId":"alpha:1:x","processDefinitionKey":"alpha","processDefinitionName":"Alpha","businessKey":"APP-10","startTime":"2026-10-23T10:00:00.000+0000","endTime":null,"state":"ACTIVE"},
                {"id":"hist-2","processDefinitionId":"alpha:1:x","processDefinitionKey":"alpha","processDefinitionName":"Alpha","businessKey":"APP-100","startTime":"2026-10-22T10:00:00.000+0000","endTime":"2026-10-22T11:00:00.000+0000","state":"COMPLETED"}]"#,
        )
        .await;
        mount(&server, "GET", "/engine-rest/history/process-instance/APP-10", 404, r#"{"message":"not found"}"#).await;

        let found = api().search_instances_by_business_key(&connection(&server), " APP-10 ").await.expect("поиск");

        assert_eq!(found.iter().map(|instance| instance.id.as_str()).collect::<Vec<_>>(), vec!["run-1", "hist-2"]);
        assert!(!found[0].ended);
        assert!(found[1].ended);
        assert_eq!(found[1].definition_name.as_deref(), Some("Alpha"));
        let runtime = requests_to(&server, "/engine-rest/process-instance").await;
        assert!(query(&runtime[0]).contains("businessKeyLike=%25APP-10%25"));
        assert!(api().search_instances_by_business_key(&connection(&server), "  ").await.is_err());
    }

    #[tokio::test]
    async fn bulk_retry_groups_incidents_into_two_requests() {
        let server = MockServer::start().await;
        mount(&server, "PUT", "/engine-rest/job/retries", 204, "").await;
        mount(&server, "PUT", "/engine-rest/external-task/retries", 204, "").await;
        let incident = |kind: &str, configuration: Option<&str>| ProcessIncident {
            id: format!("{kind}-{}", configuration.unwrap_or("none")),
            incident_type: kind.to_string(),
            activity_id: None,
            failed_activity_id: None,
            message: None,
            timestamp: None,
            configuration: configuration.map(str::to_string),
            process_definition_id: None,
            process_instance_id: None,
            execution_id: None,
            tenant_id: None,
            annotation: None,
        };
        let incidents = vec![
            incident("failedJob", Some("job-1")),
            incident("failedJob", Some("job-1")),
            incident("failedExternalTask", Some("ext-7")),
            incident("failedJob", None),
            incident("unknown", Some("x")),
        ];

        let result = api().retry_incidents(&connection(&server), &incidents, 3).await.expect("повтор");

        assert_eq!(result, BulkRetryResult { jobs: 1, external_tasks: 1, skipped: 2 });
        let jobs = requests_to(&server, "/engine-rest/job/retries").await;
        assert_eq!(body_json(&jobs[0]), json!({ "jobIds": ["job-1"], "retries": 3 }));
        let tasks = requests_to(&server, "/engine-rest/external-task/retries").await;
        assert_eq!(body_json(&tasks[0]), json!({ "externalTaskIds": ["ext-7"], "retries": 3 }));
        assert!(api().retry_incidents(&connection(&server), &[incident("unknown", None)], 3).await.is_err());
    }
}
