use std::sync::Arc;

use camunda_core::camunda::CamundaApi;
use camunda_core::models::{CamundaConnection, ProcessInstanceDetails};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::instance::InstanceView;
use crate::model;
use crate::ui;

#[derive(Clone)]
pub enum Operation {
    JobRetry(String),
    ExternalRetry(String),
    ExternalUnlock(String),
    Suspend(bool),
}

impl Operation {
    pub fn question(&self) -> String {
        match self {
            Self::JobRetry(id) => format!("Установить 3 попытки для задания движка {id}?"),
            Self::ExternalRetry(id) => format!("Установить 3 попытки для внешней задачи {id}?"),
            Self::ExternalUnlock(id) => format!("Снять блокировку с внешней задачи {id}? Другой обработчик сможет забрать её сразу."),
            Self::Suspend(true) => "Приостановить экземпляр процесса?".into(),
            Self::Suspend(false) => "Активировать экземпляр процесса?".into(),
        }
    }

    pub fn success(&self) -> &'static str {
        match self {
            Self::JobRetry(_) => "Для задания движка установлено 3 попытки.",
            Self::ExternalRetry(_) => "Для внешней задачи установлено 3 попытки.",
            Self::ExternalUnlock(_) => "Внешняя задача разблокирована.",
            Self::Suspend(true) => "Заявка приостановлена.",
            Self::Suspend(false) => "Заявка активирована.",
        }
    }

    pub async fn run(self, api: Arc<CamundaApi>, connection: CamundaConnection, instance_id: String) -> Result<(), String> {
        match self {
            Self::JobRetry(id) => api.set_job_retries(&connection, &id, 3).await,
            Self::ExternalRetry(id) => api.set_external_task_retries(&connection, &id, 3).await,
            Self::ExternalUnlock(id) => api.unlock_external_task(&connection, &id).await,
            Self::Suspend(suspended) => api.set_process_instance_suspended(&connection, &instance_id, suspended).await,
        }
    }
}

// Инциденты, задания движка и внешние задачи заявки с кнопками повтора и разблокировки.
pub fn render_diagnostics(details: &ProcessInstanceDetails, busy: bool, cx: &mut Context<InstanceView>) -> impl IntoElement {
    let row = |cx: &App| h_flex().px_4().py_3().gap_4().items_start().border_b_1().border_color(cx.theme().border);
    let empty = |text: &'static str, cx: &App| div().px_4().py_4().text_sm().text_color(cx.theme().muted_foreground).child(text);
    let incidents = ui::section(title("Инциденты", details.incidents.len(), cx), cx)
        .flex_none()
        .children(details.incidents.iter().map(|incident| {
            let meta = [
                Some(format!("тип: {}", model::incident_type_label(&incident.incident_type))),
                incident.timestamp.as_deref().map(|time| format!("время: {}", model::short_time(Some(time)))),
                incident.configuration.as_ref().map(|config| format!("конфигурация: {config}")),
                incident.annotation.as_ref().map(|note| format!("аннотация: {note}")),
                Some(format!("id: {}", incident.id)),
            ];
            row(cx).child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(cx.theme().danger).child(
                            incident
                                .failed_activity_id
                                .clone()
                                .or_else(|| incident.activity_id.clone())
                                .unwrap_or_else(|| incident.incident_type.clone()),
                        ),
                    )
                    .child(div().text_sm().child(incident.message.clone().unwrap_or_else(|| "Сообщение отсутствует".into())))
                    .child(
                        div()
                            .text_xs()
                            .font_family(crate::theme::MONO)
                            .text_color(cx.theme().muted_foreground)
                            .child(meta.into_iter().flatten().collect::<Vec<_>>().join(" · ")),
                    ),
            )
        }))
        .when(details.incidents.is_empty(), |this| this.child(empty("Открытых инцидентов нет", cx)));
    let jobs = ui::section(title("Задания движка", details.jobs.len(), cx), cx)
        .flex_none()
        .children(details.jobs.iter().map(|job| {
            let id = job.id.clone();
            row(cx)
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!("Задание · осталось попыток: {}", job.retries)),
                        )
                        .child(failure(job.exception_message.as_deref(), cx))
                        .child(
                            div()
                                .text_xs()
                                .font_family(crate::theme::MONO)
                                .text_color(cx.theme().muted_foreground)
                                .child(job.failed_activity_id.clone().unwrap_or_else(|| job.id.clone())),
                        ),
                )
                .child(
                    Button::new(SharedString::from(format!("job-{}", job.id)))
                        .label("Дать 3 попытки")
                        .primary()
                        .small()
                        .disabled(busy)
                        .on_click(cx.listener(move |this, _, _, cx| this.ask(Operation::JobRetry(id.clone()), cx))),
                )
        }))
        .when(details.jobs.is_empty(), |this| this.child(empty("Задания движка не найдены", cx)));
    let tasks = ui::section(title("Внешние задачи", details.external_tasks.len(), cx), cx)
        .flex_none()
        .children(details.external_tasks.iter().map(|task| {
            let (retry_id, unlock_id) = (task.id.clone(), task.id.clone());
            row(cx)
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(format!(
                            "{} · попыток: {}",
                            task.topic_name,
                            task.retries.map(|value| value.to_string()).unwrap_or_else(|| "—".into())
                        )))
                        .child(failure(task.error_message.as_deref(), cx))
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(format!(
                            "обработчик: {} · блокировка до: {}",
                            task.worker_id.as_deref().unwrap_or("—"),
                            model::short_time(task.lock_expiration_time.as_deref())
                        ))),
                )
                .child(
                    Button::new(SharedString::from(format!("retry-{}", task.id)))
                        .label("Дать 3 попытки")
                        .primary()
                        .small()
                        .disabled(busy)
                        .on_click(cx.listener(move |this, _, _, cx| this.ask(Operation::ExternalRetry(retry_id.clone()), cx))),
                )
                .child(
                    Button::new(SharedString::from(format!("unlock-{}", task.id)))
                        .label("Разблокировать")
                        .small()
                        .disabled(busy || task.worker_id.is_none())
                        .on_click(cx.listener(move |this, _, _, cx| this.ask(Operation::ExternalUnlock(unlock_id.clone()), cx))),
                )
        }))
        .when(details.external_tasks.is_empty(), |this| this.child(empty("Внешние задачи не найдены", cx)));
    v_flex().id("diagnostics").flex_1().min_h_0().gap_4().overflow_y_scroll().child(incidents).child(jobs).child(tasks)
}

fn title(text: &'static str, count: usize, cx: &App) -> impl IntoElement {
    h_flex().w_full().child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child(text)).child(ui::counter(count, cx))
}

fn failure(message: Option<&str>, cx: &App) -> impl IntoElement {
    match message {
        Some(message) => div().text_sm().line_clamp(2).text_color(cx.theme().danger).child(message.to_string()),
        None => div().text_sm().text_color(cx.theme().muted_foreground).child("Ошибка отсутствует"),
    }
}
