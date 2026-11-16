use camunda_core::models::DashboardWaitingActivity;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::{OverviewView, STUCK_MILLIS};
use crate::model::{self, Navigate};

impl OverviewView {
    // Строка вкладки «Ожидание»: шаг, где заявки висят дольше всего.
    pub(super) fn render_waiting(&self, item: &DashboardWaitingActivity, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = item.oldest_process_instance_id.clone();
        let tone = if item.maximum_waiting_millis >= 24 * 3_600_000 {
            Tag::danger()
        } else if item.maximum_waiting_millis >= STUCK_MILLIS {
            Tag::warning()
        } else {
            Tag::secondary()
        };
        let row_id = format!("wait-{}-{}", item.process_definition_id, item.activity_id);
        let name = HoverCard::new(SharedString::from(format!("card-{row_id}")))
            .w(px(420.))
            .trigger_style(StyleRefinement::default().w_full().min_w_0())
            .trigger(
                div()
                    .truncate()
                    .font_weight(FontWeight::MEDIUM)
                    .child(item.activity_name.clone().unwrap_or_else(|| item.activity_id.clone())),
            )
            .child(
                DescriptionList::new()
                    .columns(1)
                    .label_width(px(130.))
                    .item("Процесс", item.process_definition_id.clone(), 1)
                    .item("Шаг", item.activity_id.clone(), 1)
                    .item("Старейшая заявка", item.oldest_process_instance_id.clone(), 1)
                    .item("Ждёт с", model::short_time(item.oldest_started_at.as_deref()), 1)
                    .item("В среднем", model::format_duration(item.average_waiting_millis), 1),
            );
        h_flex()
            .id(SharedString::from(row_id))
            .px_4()
            .py_3()
            .gap_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .hover(|style| style.bg(cx.theme().table_hover))
            .cursor_pointer()
            .child(
                div()
                    .min_w(px(110.))
                    .flex_none()
                    .child(tone.small().rounded_full().child(model::format_duration(item.maximum_waiting_millis))),
            )
            .child(v_flex().flex_1().min_w_0().child(name).child(
                div().truncate().text_sm().text_color(cx.theme().muted_foreground).child(format!(
                    "{} · заявок {}",
                    item.process_definition_id.split(':').next().unwrap_or_default(),
                    item.instance_count
                )),
            ))
            .on_click(cx.listener(move |_, _, _, cx| cx.emit(Navigate::Instance(id.clone()))))
    }
}
