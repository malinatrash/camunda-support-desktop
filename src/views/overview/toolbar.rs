use camunda_core::models::DashboardDatePreset;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::{ActiveTheme, IconName};
use gpui_kit::*;

use super::OverviewView;

const PRESETS: [(DashboardDatePreset, &str); 3] =
    [(DashboardDatePreset::Live, "Сейчас"), (DashboardDatePreset::Today, "Сегодня"), (DashboardDatePreset::Yesterday, "Вчера")];

impl OverviewView {
    // Заголовок экрана: период, время обновления и кнопка обновить.
    pub(super) fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status: SharedString = if self.load.is_some() && self.dashboard.is_none() {
            "загружаем…".into()
        } else {
            self.refreshed_at.map(|at| format!("обновлено {} с назад", at.elapsed().as_secs())).unwrap_or_default().into()
        };
        let selected = PRESETS.iter().position(|(preset, _)| *preset == self.preset).unwrap_or_default();
        let presets =
            TabBar::new("overview-preset").segmented().selected_index(selected).children(PRESETS.map(|(_, label)| label)).on_click(
                cx.listener(|this, index: &usize, _, cx| {
                    this.preset = PRESETS[*index].0;
                    this.dashboard = None;
                    this.refresh(cx);
                }),
            );
        Toolbar::new("overview-toolbar")
            .w_full()
            .p_0()
            .gap_2()
            .content(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Обзор"))
            .content(div().flex_1())
            .content(div().text_sm().text_color(cx.theme().muted_foreground).child(status))
            .child(presets)
            .child(
                Button::new("refresh")
                    .icon(IconName::RefreshCw)
                    .ghost()
                    .loading(self.load.is_some())
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
            )
    }
}
