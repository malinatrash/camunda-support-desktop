use camunda_core::models::ProcessInstanceDetails;
use gpui_kit::component::table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::hover;
use super::panel::{empty, list_panel};
use crate::model;

// Хронология прохождения кубиков по History API таблицей с прокруткой.
pub(super) fn render(details: &ProcessInstanceDetails, cx: &App) -> impl IntoElement {
    let head = v_flex().child(div().font_weight(FontWeight::SEMIBOLD).child("Хронология прохождения")).child(
        div().text_sm().text_color(cx.theme().muted_foreground).child(format!("{} событий · History API", details.activity_timeline.len())),
    );
    let columns = TableRow::new()
        .child(TableHead::new().col_span(2).child("Начало"))
        .child(TableHead::new().col_span(3).child("Кубик"))
        .child(TableHead::new().col_span(2).child("Тип"))
        .child(TableHead::new().text_right().child("Длительность"))
        .child(TableHead::new().text_right().child("Статус"));
    let rows = details.activity_timeline.iter().map(|entry| {
        let status = if entry.canceled {
            Tag::warning().small().rounded_full().child("отменён")
        } else if entry.end_time.is_none() {
            Tag::secondary().small().rounded_full().child("активен")
        } else {
            Tag::success().small().rounded_full().child("выполнен")
        };
        TableRow::new()
            .child(
                TableCell::new()
                    .col_span(2)
                    .font_family(crate::platform::theme::mono(cx))
                    .text_xs()
                    .child(model::short_time(entry.start_time.as_deref())),
            )
            .child(
                TableCell::new().col_span(3).child(hover::fields_card(
                    SharedString::from(format!("timeline-{}", entry.id)),
                    v_flex()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .font_weight(FontWeight::MEDIUM)
                                .child(entry.activity_name.clone().unwrap_or_else(|| entry.activity_id.clone())),
                        )
                        .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(entry.activity_id.clone())),
                    vec![
                        ("Кубик", entry.activity_id.clone()),
                        ("Тип", entry.activity_type.clone()),
                        ("Начало", model::short_time(entry.start_time.as_deref())),
                        ("Окончание", model::short_time(entry.end_time.as_deref())),
                        ("Длительность", entry.duration_millis.map(format_millis).unwrap_or_else(|| "—".into())),
                        ("ID экземпляра", entry.id.clone()),
                    ],
                )),
            )
            .child(
                TableCell::new()
                    .col_span(2)
                    .text_color(cx.theme().muted_foreground)
                    .child(div().truncate().child(entry.activity_type.clone())),
            )
            .child(TableCell::new().text_right().child(entry.duration_millis.map(format_millis).unwrap_or_else(|| "—".into())))
            .child(TableCell::new().text_right().child(status))
    });
    list_panel(head, cx).child(
        div()
            .id("timeline")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(Table::new().child(TableHeader::new().child(columns)).child(TableBody::new().children(rows)))
            .when(details.activity_timeline.is_empty(), |this| this.child(empty("Событий нет"))),
    )
}

fn format_millis(millis: i64) -> String {
    if millis < 1_000 {
        format!("{millis} мс")
    } else if millis < 60_000 {
        format!("{:.1} с", millis as f64 / 1_000.)
    } else {
        model::format_duration(millis)
    }
}
