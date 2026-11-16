use camunda_core::models::ProcessInstanceDetails;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants as _};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::panel::{counted, empty};
use super::{InstanceView, hover};
use crate::model;

impl InstanceView {
    // Боковая панель схемы: выбранный кубик, текущие позиции и инциденты.
    pub(super) fn render_side(&self, details: &ProcessInstanceDetails, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("side")
            .size_full()
            .pl_4()
            .gap_4()
            .overflow_y_scroll()
            .child(self.render_selected(details, cx))
            .child(
                GroupBox::new()
                    .outline()
                    .title(counted(tr!("instance.side.current"), details.active_activities.len()))
                    .content_style(crate::platform::theme::panel())
                    .flex_none()
                    .children(details.active_activities.iter().map(|activity| {
                        v_flex()
                            .px_4()
                            .py_3()
                            .gap_1p5()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(activity.activity_name.clone().unwrap_or_else(|| activity.activity_id.clone())),
                            )
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .flex_wrap()
                                    .child(Tag::secondary().small().rounded_full().child(tr!("instance.side.active")))
                                    .when(!activity.incident_ids.is_empty(), |this| {
                                        this.child(Tag::danger().small().rounded_full().child(tr!("instance.side.incident")))
                                    })
                                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child(activity.activity_type.clone())),
                            )
                    })),
            )
            .child(
                GroupBox::new()
                    .outline()
                    .title(counted(tr!("instance.side.incidents"), details.incidents.len()))
                    .content_style(crate::platform::theme::panel().p_3().gap_2())
                    .flex_none()
                    .children(details.incidents.iter().map(|incident| {
                        let activity = incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_default();
                        let message = incident.message.clone().unwrap_or_else(|| tr!("instance.side.no_message").into());
                        let meta = format!(
                            "{} · {}",
                            model::incident_type_label(&incident.incident_type),
                            model::short_time(incident.timestamp.as_deref())
                        );
                        hover::text_card(
                            SharedString::from(format!("side-incident-card-{}", incident.id)),
                            Alert::error(SharedString::from(format!("side-incident-{}", incident.id)), format!("{message}\n{meta}"))
                                .small()
                                .title(activity.clone()),
                            activity,
                            format!("{message}\n\n{meta}"),
                        )
                    }))
                    .when(details.incidents.is_empty(), |this| this.child(empty(tr!("instance.side.no_incidents")))),
            )
    }

    fn render_selected(&self, details: &ProcessInstanceDetails, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(node) = self.selected_node.as_deref().and_then(|id| details.diagram.node(id)) else {
            return GroupBox::new()
                .outline()
                .title(div().text_sm().text_color(cx.theme().muted_foreground).child(tr!("instance.side.pick_node")))
                .content_style(crate::platform::theme::panel())
                .flex_none();
        };
        let history = details.activity_history.iter().find(|item| item.activity_id == node.id);
        let target = node.id.clone();
        let teleportable = details.diagram.teleport_targets().any(|candidate| candidate.id == node.id);
        GroupBox::new()
            .outline()
            .title(div().font_weight(FontWeight::SEMIBOLD).truncate().child(node.name.clone().unwrap_or_else(|| node.id.clone())))
            .content_style(crate::platform::theme::panel().p_4().gap_2())
            .flex_none()
            .text_sm()
            .child(div().text_color(cx.theme().muted_foreground).child(format!("{} · {}", node.node_type, node.id)))
            .children(history.map(|item| {
                tr!("instance.side.history", completed = item.completed_count, active = item.active_count, canceled = item.canceled_count)
            }))
            .children(node.topic.clone().map(|topic| tr!("instance.side.topic", topic = topic)))
            .when(teleportable, |this| {
                this.child(
                    Button::new("teleport-here")
                        .label(tr!("instance.side.teleport_here"))
                        .primary()
                        .small()
                        .disabled(!self.can_teleport())
                        .on_click(cx.listener(move |this, _, window, cx| this.open_teleport(Some(target.clone()), window, cx))),
                )
            })
    }
}
