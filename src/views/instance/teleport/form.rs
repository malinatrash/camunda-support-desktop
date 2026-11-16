use camunda_core::models::Environment;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::{ActiveTheme, Disableable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{TeleportEvent, TeleportView};

impl Render for TeleportView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let production = self.connection.environment == Environment::Production;
        let can_execute = self.run.is_none()
            && self.source.is_some()
            && self.target.is_some()
            && !self.annotation.read(cx).value().trim().is_empty()
            && (!production || self.confirmed);
        self.sync_placeholders(window, cx);
        v_flex()
            .h(window.viewport_size().height * 0.65)
            .gap_4()
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(tr!("instance.teleport.intro")))
            .child(h_flex().flex_1().min_h_0().gap_4().child(self.render_sources(cx)).child(self.render_targets(cx)))
            .child(
                v_flex()
                    .gap_1p5()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(tr!("instance.teleport.annotation")))
                    .child(Input::new(&self.annotation)),
            )
            .when(production, |this| {
                this.child(
                    Checkbox::new("confirm-production")
                        .label(tr!("instance.teleport.confirm_production"))
                        .checked(self.confirmed)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.confirmed = *checked;
                            cx.notify();
                        })),
                )
            })
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("teleport-error", error)))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel")
                            .label(tr!("instance.teleport.cancel"))
                            .ghost()
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(TeleportEvent::Cancel))),
                    )
                    .child(
                        Button::new("execute")
                            .label(tr!("instance.teleport.execute"))
                            .map(|button| if production { button.danger() } else { button.primary() })
                            .loading(self.run.is_some())
                            .disabled(!can_execute)
                            .on_click(cx.listener(|this, _, _, cx| this.execute(cx))),
                    ),
            )
    }
}
