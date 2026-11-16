use std::time::Instant;

use camunda_core::models::DashboardDateFilter;
use gpui_kit::*;

use super::OverviewView;
use crate::model;
use crate::platform::services;

impl OverviewView {
    // Загрузка дашборда за выбранный период и пересборка списков.
    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let filter = DashboardDateFilter { preset: self.preset, ..Default::default() };
        let request = services::request(cx, move |api| async move { api.load_dashboard(&connection, &filter).await });
        self.load = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(dashboard) => {
                        this.incident_groups = model::group_incidents(&dashboard.incidents);
                        this.dashboard = Some(dashboard);
                        this.error = None;
                        this.refreshed_at = Some(Instant::now());
                        this.regroup(cx);
                    }
                    Err(error) => this.error = Some(error),
                }
                this.load = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    // Процессы по тексту из поля фильтра.
    pub(super) fn regroup(&mut self, cx: &App) {
        let Some(dashboard) = &self.dashboard else { return };
        let needle = self.filter.read(cx).value().trim().to_lowercase();
        let definitions: Vec<_> = dashboard
            .definitions
            .iter()
            .filter(|d| {
                needle.is_empty()
                    || d.key.to_lowercase().contains(&needle)
                    || d.name.as_deref().is_some_and(|name| name.to_lowercase().contains(&needle))
            })
            .cloned()
            .collect();
        self.processes = model::group_processes(&definitions);
    }
}
