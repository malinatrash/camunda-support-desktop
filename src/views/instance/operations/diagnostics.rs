use camunda_core::models::ProcessInstanceDetails;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants as _};
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::Operation;
use crate::model;
use crate::views::instance::panel::{counted, empty};
use crate::views::instance::{InstanceView, hover};

// Инциденты, задания движка и внешние задачи заявки с кнопками повтора и разблокировки.
pub(in crate::views::instance) fn render_diagnostics(
    details: &ProcessInstanceDetails,
    busy: bool,
    cx: &mut Context<InstanceView>,
) -> impl IntoElement {
    let incidents = details.incidents.iter().map(|incident| {
        let meta = [
            Some(format!("тип: {}", model::incident_type_label(&incident.incident_type))),
            incident.timestamp.as_deref().map(|time| format!("время: {}", model::short_time(Some(time)))),
            incident.configuration.as_ref().map(|config| format!("конфигурация: {config}")),
            incident.annotation.as_ref().map(|note| format!("аннотация: {note}")),
            Some(format!("id: {}", incident.id)),
        ];
        let activity =
            incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_else(|| incident.incident_type.clone());
        let message = incident.message.clone().unwrap_or_else(|| "Сообщение отсутствует".into());
        let meta = meta.into_iter().flatten().collect::<Vec<_>>().join(" · ");
        TableRow::new()
            .child(TableCell::new().col_span(2).text_color(cx.theme().danger).font_weight(FontWeight::SEMIBOLD).child(activity.clone()))
            .child(TableCell::new().col_span(5).child(hover::text_card(
                SharedString::from(format!("incident-{}", incident.id)),
                text_with_meta(message.clone(), meta.clone(), cx),
                activity,
                format!("{message}\n\n{meta}"),
            )))
    });
    let jobs = details.jobs.iter().map(|job| {
        let id = job.id.clone();
        TableRow::new()
            .child(TableCell::new().col_span(2).child(text_with_meta(
                format!("Задание · осталось попыток: {}", job.retries),
                job.failed_activity_id.clone().unwrap_or_else(|| job.id.clone()),
                cx,
            )))
            .child(TableCell::new().col_span(4).child(failure(format!("job-error-{}", job.id), job.exception_message.as_deref(), cx)))
            .child(
                TableCell::new().text_right().child(
                    Button::new(SharedString::from(format!("job-{}", job.id)))
                        .label("Дать 3 попытки")
                        .outline()
                        .small()
                        .disabled(busy)
                        .on_click(cx.listener(move |this, _, window, cx| this.ask(Operation::JobRetry(id.clone()), window, cx))),
                ),
            )
    });
    let tasks =
        details.external_tasks.iter().map(|task| {
            let (retry_id, unlock_id) = (task.id.clone(), task.id.clone());
            let retries = task.retries.map(|value| value.to_string()).unwrap_or_else(|| "—".into());
            let worker = format!(
                "обработчик: {} · блокировка до: {}",
                task.worker_id.as_deref().unwrap_or("—"),
                model::short_time(task.lock_expiration_time.as_deref())
            );
            TableRow::new()
                .child(TableCell::new().col_span(2).child(text_with_meta(format!("{} · попыток: {retries}", task.topic_name), worker, cx)))
                .child(TableCell::new().col_span(3).child(failure(format!("task-error-{}", task.id), task.error_message.as_deref(), cx)))
                .child(
                    TableCell::new().col_span(2).text_right().child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new(SharedString::from(format!("retry-{}", task.id)))
                                    .label("Дать 3 попытки")
                                    .outline()
                                    .small()
                                    .disabled(busy)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.ask(Operation::ExternalRetry(retry_id.clone()), window, cx)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("unlock-{}", task.id)))
                                    .label("Разблокировать")
                                    .ghost()
                                    .small()
                                    .disabled(busy || task.worker_id.is_none())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.ask(Operation::ExternalUnlock(unlock_id.clone()), window, cx)
                                    })),
                            ),
                    ),
                )
        });
    v_flex()
        .id("diagnostics")
        .flex_1()
        .min_h_0()
        .gap_4()
        .overflow_y_scroll()
        .child(section("Инциденты", ["Кубик", "Сообщение"], [2, 5], incidents.collect(), "Открытых инцидентов нет"))
        .child(section("Задания движка", ["Задание", "Ошибка", ""], [2, 4, 1], jobs.collect(), "Задания движка не найдены"))
        .child(section("Внешние задачи", ["Топик", "Ошибка", ""], [2, 3, 2], tasks.collect(), "Внешние задачи не найдены"))
}

fn section<const N: usize>(
    title: &'static str,
    heads: [&'static str; N],
    spans: [usize; N],
    rows: Vec<TableRow>,
    none: &'static str,
) -> GroupBox {
    let count = rows.len();
    let header = TableRow::new().children(heads.into_iter().zip(spans).map(|(head, span)| TableHead::new().col_span(span).child(head)));
    let body = if rows.is_empty() {
        empty(none).into_any_element()
    } else {
        Table::new().small().child(TableHeader::new().child(header)).child(TableBody::new().children(rows)).into_any_element()
    };
    GroupBox::new().outline().title(counted(title, count)).content_style(crate::platform::theme::panel()).flex_none().child(body)
}

fn text_with_meta(text: String, meta: String, cx: &App) -> Div {
    v_flex()
        .min_w_0()
        .gap_1()
        .child(div().text_sm().child(text))
        .child(div().text_xs().font_family(crate::platform::theme::mono(cx)).text_color(cx.theme().muted_foreground).child(meta))
}

fn failure(id: String, message: Option<&str>, cx: &App) -> AnyElement {
    match message {
        Some(message) => hover::text_card(
            SharedString::from(id),
            div().text_sm().line_clamp(2).text_color(cx.theme().danger).child(message.to_string()),
            "Ошибка".into(),
            message.to_string(),
        )
        .into_any_element(),
        None => div().text_sm().text_color(cx.theme().muted_foreground).child("Ошибка отсутствует").into_any_element(),
    }
}
