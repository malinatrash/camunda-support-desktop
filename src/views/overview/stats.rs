use camunda_core::models::{DashboardDatePreset, ProcessDashboard};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{OverviewView, STUCK_MILLIS};
use crate::model;

impl OverviewView {
    // Сводные цифры над панелями.
    pub(super) fn render_stats(&self, dashboard: &ProcessDashboard, cx: &App) -> impl IntoElement {
        let incidents: i32 = dashboard.definitions.iter().map(|d| d.incidents).sum();
        let instances: i32 = dashboard.definitions.iter().map(|d| d.instances).sum();
        let stuck: i32 = dashboard
            .longest_waiting_activities
            .iter()
            .filter(|item| item.maximum_waiting_millis >= STUCK_MILLIS)
            .map(|item| item.instance_count)
            .sum();
        let live = self.preset == DashboardDatePreset::Live;
        let value = |value: i32, color: Option<Hsla>| {
            div()
                .text_size(px(26.))
                .font_weight(FontWeight::SEMIBOLD)
                .when_some(color, |this, color| this.text_color(color))
                .child(model::grouped(value as i64))
                .into_any_element()
        };
        DescriptionList::vertical()
            .bordered(true)
            .columns(4)
            .item("инцидентов", value(incidents, (incidents > 0).then_some(cx.theme().danger)), 1)
            .item(if live { "в работе" } else { "стартов" }, value(instances, None), 1)
            .item("висят дольше 4 ч", value(stuck, (stuck > 0).then_some(cx.theme().warning)), 1)
            .item("завершено", value(dashboard.completed_instances, None), 1)
    }
}
