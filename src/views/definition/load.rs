use camunda_core::models::DashboardDateFilter;
use gpui_kit::*;

use super::DefinitionView;
use crate::platform::services;

impl DefinitionView {
    // Загрузка заявок выбранной версии или всех версий.
    pub(super) fn reload(&mut self, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let id = self.version_id.clone().unwrap_or_else(|| self.definition_id.clone());
        let all_versions = self.version_id.is_none();
        let request = services::request(cx, move |api| async move {
            api.load_process_definition_details(&connection, &id, &DashboardDateFilter::default(), all_versions).await
        });
        self.table.update(cx, |table, cx| {
            table.delegate_mut().loading = true;
            table.refresh(cx);
        });
        self.load = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| {
                this.table.update(cx, |table, cx| {
                    let delegate = table.delegate_mut();
                    delegate.loading = false;
                    if let Ok(details) = &result {
                        delegate.rows = details.instances.clone();
                    }
                    table.refresh(cx);
                });
                match result {
                    Ok(details) => {
                        this.details = Some(details);
                        this.error = None;
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
}
