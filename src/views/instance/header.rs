use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, IconName, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{InstanceView, hover};
use crate::model::{self, Navigate};

impl InstanceView {
    // Шапка заявки: назад, бизнес-ключ с состоянием и панель действий.
    pub(super) fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let key = self.details.as_ref().and_then(|details| details.instance.business_key.clone());
        let copy_key = key.clone();
        let definition = self.details.as_ref().map(|details| details.instance.definition_key.clone()).unwrap_or_default();
        h_flex()
            .gap_3()
            .child(
                Button::new("back")
                    .icon(IconName::ChevronLeft)
                    .ghost()
                    .small()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Navigate::Back))),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(hover::fields_card(
                                "business-key-card",
                                div()
                                    .id("business-key")
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .hover(|style| style.underline())
                                    .child(key.unwrap_or_else(|| "Заявка".into()))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(key) = copy_key.clone() {
                                            this.copy(key, "Бизнес-ключ скопирован.", cx);
                                        }
                                    })),
                                self.instance_fields(),
                            ))
                            .when_some(self.details.as_ref(), |this, details| {
                                this.child(model::state_tag(details.instance.suspended, details.instance.ended)).when(
                                    !details.incidents.is_empty(),
                                    |this| {
                                        this.child(Tag::danger().small().rounded_full().child(format!("{} инц.", details.incidents.len())))
                                    },
                                )
                            }),
                    )
                    .child(hover::fields_card(
                        "instance-id-card",
                        div()
                            .truncate()
                            .text_xs()
                            .font_family(crate::platform::theme::mono(cx))
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{definition} · {}", self.instance_id)),
                        self.instance_fields(),
                    )),
            )
            .child(self.render_actions(cx))
    }

    fn instance_fields(&self) -> Vec<(&'static str, String)> {
        let Some(instance) = self.details.as_ref().map(|details| &details.instance) else {
            return vec![("ID заявки", self.instance_id.clone())];
        };
        let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| "—".into());
        vec![
            ("ID заявки", instance.id.clone()),
            ("Бизнес-ключ", optional(&instance.business_key)),
            ("Процесс", instance.definition_name.clone().unwrap_or_else(|| instance.definition_key.clone())),
            ("Определение", instance.definition_id.clone()),
            ("Старт", model::short_time(instance.start_time.as_deref())),
            ("Окончание", model::short_time(instance.end_time.as_deref())),
            ("Тенант", optional(&instance.tenant_id)),
        ]
    }
}
