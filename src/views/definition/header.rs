use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::{ActiveTheme, IconName, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::DefinitionView;
use crate::model::{self, Navigate};
use crate::views::incidents::plural_key;

impl DefinitionView {
    // Путь «Обзор / процесс / версия», название и действия.
    pub(super) fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let key = self.definition_id.split(':').next().unwrap_or_default().to_string();
        let title =
            self.details.as_ref().map(|details| model::display_name(&details.definition).to_string()).unwrap_or_else(|| key.clone());
        let version = match &self.version_id {
            Some(id) => format!("v{}", id.split(':').nth(1).unwrap_or("?")),
            None => tr!("definition.all_versions").to_string(),
        };
        let instances = self.details.as_ref().map(|details| details.instances.as_slice()).unwrap_or_default();
        let with_incidents = instances.iter().filter(|item| item.incident_count > 0).count();
        let path = Breadcrumb::new()
            .min_w_0()
            .overflow_hidden()
            .child(BreadcrumbItem::new(tr!("definition.overview")).on_click(cx.listener(|_, _, _, cx| cx.emit(Navigate::Back))))
            .child(title.clone())
            .child(version);
        let actions = Toolbar::new("definition-actions")
            .p_0()
            .gap_2()
            .content(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr!(plural_key("definition.instances", instances.len()), count = instances.len())),
            )
            .contents((with_incidents > 0).then(|| {
                Tag::danger().small().rounded_full().child(tr!("definition.with_incidents", count = with_incidents)).into_any_element()
            }))
            .child(
                Button::new("reload")
                    .icon(IconName::RefreshCw)
                    .ghost()
                    .loading(self.load.is_some())
                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            );
        v_flex().gap_2().child(path).child(
            h_flex()
                .gap_3()
                .child(
                    v_flex().flex_1().min_w_0().child(div().truncate().text_lg().font_weight(FontWeight::SEMIBOLD).child(title)).child(
                        div()
                            .truncate()
                            .text_xs()
                            .font_family(crate::platform::theme::mono(cx))
                            .text_color(cx.theme().muted_foreground)
                            .child(key),
                    ),
                )
                .child(actions),
        )
    }
}
