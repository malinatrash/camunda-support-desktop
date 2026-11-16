use camunda_core::models::{CamundaConnection, Environment, ProcessIncident};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::{Disableable, WindowExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::IncidentsView;
use crate::platform::services;

const RETRIES: i32 = 3;

impl IncidentsView {
    // Открывает подтверждение повтора; диалог живёт своим состоянием до завершения запроса.
    pub(super) fn ask_retry(&mut self, incidents: Vec<ProcessIncident>, window: &mut Window, cx: &mut Context<Self>) {
        let description = format!(
            "Выставить {RETRIES} попытки для {} инцидентов? Camunda сразу начнёт повторное выполнение заданий и внешних задач.",
            incidents.len()
        );
        let view = cx.entity().downgrade();
        let confirm = cx.new(|_| RetryConfirm {
            connection: self.connection.clone(),
            production: self.connection.environment == Environment::Production,
            incidents,
            view,
            confirmed: false,
            error: None,
            task: None,
        });
        window.open_alert_dialog(cx, move |alert, _, _| {
            alert.title("Повторить инциденты").description(description.clone()).width(px(520.)).keyboard(false).footer(confirm.clone())
        });
    }
}

pub(super) fn retryable(incident: &ProcessIncident) -> bool {
    matches!(incident.incident_type.as_str(), "failedJob" | "failedExternalTask") && incident.configuration.is_some()
}

struct RetryConfirm {
    connection: CamundaConnection,
    production: bool,
    incidents: Vec<ProcessIncident>,
    view: WeakEntity<IncidentsView>,
    confirmed: bool,
    error: Option<String>,
    task: Option<Task<()>>,
}

impl RetryConfirm {
    fn run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let incidents = self.incidents.clone();
        let request = services::request(cx, move |api| async move { api.retry_incidents(&connection, &incidents, RETRIES).await });
        self.error = None;
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                this.task = None;
                match result {
                    Ok(result) => {
                        let skipped = if result.skipped > 0 { format!(", пропущено {}", result.skipped) } else { String::new() };
                        let notice = format!("Повтор запущен: заданий {}, внешних задач {}{skipped}.", result.jobs, result.external_tasks);
                        window.close_dialog(cx);
                        this.view
                            .update(cx, |view, cx| {
                                view.notice = Some(notice.into());
                                view.table.update(cx, |table, _| table.delegate_mut().selected.clear());
                                view.reload(window, cx);
                            })
                            .ok();
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

impl Render for RetryConfirm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let running = self.task.is_some();
        v_flex()
            .w_full()
            .gap_4()
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("retry-error", error)))
            .when(self.production, |this| {
                this.child(Checkbox::new("confirm-production").label("Понимаю, что это прод").checked(self.confirmed).on_click(
                    cx.listener(|this, checked: &bool, _, cx| {
                        this.confirmed = *checked;
                        cx.notify();
                    }),
                ))
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("retry-cancel")
                            .label("Отмена")
                            .outline()
                            .disabled(running)
                            .on_click(cx.listener(|_, _, window, cx| window.close_dialog(cx))),
                    )
                    .child(
                        Button::new("retry-run")
                            .label("Повторить")
                            .map(|button| if self.production { button.danger() } else { button.primary() })
                            .loading(running)
                            .disabled(running || (self.production && !self.confirmed))
                            .on_click(cx.listener(|this, _, window, cx| this.run(window, cx))),
                    ),
            )
    }
}
