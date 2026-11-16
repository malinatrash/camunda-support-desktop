use camunda_core::models::ProcessDashboard;
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyTitle};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, v_flex};
use gpui_kit::*;

use super::{OverviewView, TAB_ERRORS};

impl OverviewView {
    // Панель с вкладками «Ошибки» и «Ожидание».
    pub(super) fn render_errors(&self, dashboard: &ProcessDashboard, cx: &mut Context<Self>) -> impl IntoElement {
        let count = |n: usize| Tag::secondary().small().rounded_full().child(n.to_string());
        let tabs = TabBar::new("overview-tabs")
            .underline()
            .selected_index(self.tab)
            .child(Tab::new().label("Ошибки").suffix(count(self.incident_groups.len())))
            .child(Tab::new().label("Ожидание").suffix(count(dashboard.longest_waiting_activities.len())))
            .on_click(cx.listener(|this, index: &usize, _, cx| {
                this.tab = *index;
                cx.notify();
            }));
        let body = div().id("errors").flex_1().min_h_0().overflow_y_scroll();
        let body = if self.tab != TAB_ERRORS {
            let rows: Vec<_> = dashboard.longest_waiting_activities.iter().map(|item| self.render_waiting(item, cx)).collect();
            body.children(rows)
        } else if self.incident_groups.is_empty() {
            body.child(Empty::new().header(EmptyHeader::new().title(EmptyTitle::new().child(
                if self.preset == camunda_core::models::DashboardDatePreset::Live {
                    "За последние 24 часа инцидентов нет"
                } else {
                    "Инцидентов нет"
                },
            ))))
        } else {
            body.child(self.render_incidents(cx))
        };
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .child(div().px_2().child(tabs))
            .child(body)
    }
}
