use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::sidebar::SidebarToggleButton;
use gpui_kit::component::{Sizable, TitleBar, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::AppView;

impl AppView {
    // Верхняя полоса над содержимым: кнопка боковой панели, когда она свёрнута, и поиск ⌘K.
    pub(super) fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            .when(!self.sidebar_collapsed, |bar| bar.pl_3())
            .bg(crate::platform::theme::surface(cx))
            .border_color(transparent_black())
            .child(
                h_flex()
                    .w_full()
                    .pr_3()
                    .gap_2()
                    .when(self.sidebar_collapsed, |this| this.child(self.sidebar_toggle(cx)))
                    .child(div().flex_1())
                    .child(
                        Button::new("palette-trigger")
                            .ghost()
                            .small()
                            .icon(Lucide::Search)
                            .child(h_flex().gap_3().child("Найти заявку").child(Kbd::new(Keystroke::parse("cmd-k").expect("сочетание"))))
                            .on_click(cx.listener(|this, _, window, cx| this.toggle_palette(window, cx))),
                    ),
            )
    }

    pub(super) fn sidebar_toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        SidebarToggleButton::new().collapsed(self.sidebar_collapsed).on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx)))
    }

    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }
}
