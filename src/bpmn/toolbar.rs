use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, IconName, Sizable, h_flex};
use gpui_kit::*;

use super::BpmnView;

// Кнопки масштаба в правом верхнем углу схемы.
pub fn toolbar(zoom_label: String, cx: &mut Context<BpmnView>) -> impl IntoElement {
    h_flex()
        .absolute()
        .top_2()
        .right_2()
        .gap_1()
        .p_1()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().popover)
        .child(Button::new("zoom-out").icon(IconName::Minus).ghost().xsmall().on_click(cx.listener(|this, _, _, cx| {
            let center = this.center_of();
            this.zoom_at(0.8, center, cx)
        })))
        .child(Button::new("fit").label(tr!("bpmn.fit")).ghost().xsmall().on_click(cx.listener(|this, _, _, cx| {
            this.transform = None;
            cx.notify();
        })))
        .child(Button::new("zoom-in").icon(IconName::Plus).ghost().xsmall().on_click(cx.listener(|this, _, _, cx| {
            let center = this.center_of();
            this.zoom_at(1.25, center, cx)
        })))
        .child(div().w(px(44.)).text_xs().text_right().text_color(cx.theme().muted_foreground).child(zoom_label))
}
