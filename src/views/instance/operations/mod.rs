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
                .title("Подтвердите действие")
                .description(operation.question())
                .when(production, |alert| alert.child(Tag::danger().small().rounded_full().child("Это прод")))
                .ok_text("Выполнить")
                .cancel_text("Отмена")
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
                        this.notice = Some(success.into());
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
    fn question(&self) -> String {
        match self {
            Self::JobRetry(id) => format!("Установить 3 попытки для задания движка {id}?"),
            Self::ExternalRetry(id) => format!("Установить 3 попытки для внешней задачи {id}?"),
            Self::ExternalUnlock(id) => format!("Снять блокировку с внешней задачи {id}? Другой обработчик сможет забрать её сразу."),
            Self::Suspend(true) => "Приостановить экземпляр процесса?".into(),
            Self::Suspend(false) => "Активировать экземпляр процесса?".into(),
        }
    }

    fn success(&self) -> &'static str {
        match self {
            Self::JobRetry(_) => "Для задания движка установлено 3 попытки.",
            Self::ExternalRetry(_) => "Для внешней задачи установлено 3 попытки.",
            Self::ExternalUnlock(_) => "Внешняя задача разблокирована.",
            Self::Suspend(true) => "Заявка приостановлена.",
            Self::Suspend(false) => "Заявка активирована.",
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
