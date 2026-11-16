mod diagnostics;

use std::sync::Arc;

use camunda_core::camunda::CamundaApi;
use camunda_core::models::{CamundaConnection, Environment};
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::tag::Tag;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::InstanceView;
use crate::platform::services;
pub(super) use diagnostics::render_diagnostics;

#[derive(Clone)]
pub enum Operation {
    JobRetry(String),
    ExternalRetry(String),
    ExternalUnlock(String),
    Suspend(bool),
}

impl InstanceView {
    // Спрашивает подтверждение операции в диалоге и запускает её по «Выполнить».
    pub(super) fn ask(&mut self, operation: Operation, window: &mut Window, cx: &mut Context<Self>) {
        let production = self.connection.environment == Environment::Production;
        let view = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (view, operation) = (view.clone(), operation.clone());
            alert
                .confirm()
                .title(tr!("instance.op.confirm_title"))
                .description(operation.question())
                .when(production, |alert| alert.child(Tag::danger().small().rounded_full().child(tr!("instance.op.production"))))
                .ok_text(tr!("instance.op.run"))
                .cancel_text(tr!("instance.op.cancel"))
                .ok_variant(if production { ButtonVariant::Danger } else { ButtonVariant::Primary })
                .on_ok(move |_, window, cx| {
                    view.update(cx, |this, cx| this.run_operation(operation.clone(), window, cx)).ok();
                    true
                })
        });
    }

    fn run_operation(&mut self, operation: Operation, window: &mut Window, cx: &mut Context<Self>) {
        let success = operation.success();
        let (connection, instance_id) = (self.connection.clone(), self.instance_id.clone());
        let request = services::request(cx, move |api| operation.run(api, connection, instance_id));
        self.error = None;
        self.operation = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                this.operation = None;
                match result {
                    Ok(()) => {
                        this.notice = Some(success);
                        this.reload(window, cx);
                    }
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }
}

impl Operation {
    fn question(&self) -> SharedString {
        match self {
            Self::JobRetry(id) => tr!("instance.op.job_retry_question", id = id),
            Self::ExternalRetry(id) => tr!("instance.op.external_retry_question", id = id),
            Self::ExternalUnlock(id) => tr!("instance.op.external_unlock_question", id = id),
            Self::Suspend(true) => tr!("instance.op.suspend_question"),
            Self::Suspend(false) => tr!("instance.op.activate_question"),
        }
    }

    fn success(&self) -> SharedString {
        match self {
            Self::JobRetry(_) => tr!("instance.op.job_retry_done"),
            Self::ExternalRetry(_) => tr!("instance.op.external_retry_done"),
            Self::ExternalUnlock(_) => tr!("instance.op.external_unlock_done"),
            Self::Suspend(true) => tr!("instance.op.suspend_done"),
            Self::Suspend(false) => tr!("instance.op.activate_done"),
        }
    }

    async fn run(self, api: Arc<CamundaApi>, connection: CamundaConnection, instance_id: String) -> Result<(), String> {
        match self {
            Self::JobRetry(id) => api.set_job_retries(&connection, &id, 3).await,
            Self::ExternalRetry(id) => api.set_external_task_retries(&connection, &id, 3).await,
            Self::ExternalUnlock(id) => api.unlock_external_task(&connection, &id).await,
            Self::Suspend(suspended) => api.set_process_instance_suspended(&connection, &instance_id, suspended).await,
        }
    }
}
