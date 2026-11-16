use camunda_core::models::CamundaConnection;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::avatar::Avatar;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::sidebar::SidebarHeader;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::AppView;
use super::palette::Section;
use crate::platform::{environment, motion};

impl AppView {
    // Переключатель подключения: шапка боковой панели открывает сетку карточек.
    pub(super) fn render_connection_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let connections = self.connections.clone();
        let current = self.connection.as_ref().map(|connection| connection.id.clone());
        let view = cx.entity().downgrade();
        let trigger = SidebarHeader::new()
            .child(match &self.connection {
                Some(connection) => connection_badge(connection, cx).into_any_element(),
                None => div().text_sm().child(tr!("shell.no_connection")).into_any_element(),
            })
            .child(Icon::new(Lucide::ChevronsUpDown).small().text_color(cx.theme().muted_foreground));
        let generation = self.popover_generation;
        let opened = cx.entity().downgrade();
        Popover::new("connections")
            .trigger(trigger)
            .on_open_change(move |open, _, cx| {
                if *open {
                    opened.update(cx, |this, _| this.popover_generation += 1).ok();
                }
            })
            .content(move |_, _, cx| {
                let columns = connections.len().clamp(1, 3) as u16;
                motion::enter(
                    "connections-grid",
                    generation,
                    v_flex()
                        .gap_3()
                        .w(px(240. * columns as f32))
                        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(tr!("shell.connections")))
                        .child(div().grid().grid_cols(columns).gap_2().children(connections.iter().map(|connection| {
                            let active = current.as_deref() == Some(connection.id.as_str());
                            let (target, view) = (connection.clone(), view.clone());
                            connection_card(connection, active, cx).on_click(cx.listener(move |state, _, window, cx| {
                                view.update(cx, |this, cx| this.select_connection(target.clone(), window, cx)).ok();
                                state.dismiss(window, cx);
                            }))
                        })))
                        .child({
                            let view = view.clone();
                            Button::new("manage-connections")
                                .ghost()
                                .small()
                                .icon(Lucide::Settings)
                                .label(tr!("shell.manage_connections"))
                                .on_click(cx.listener(move |state, _, window, cx| {
                                    view.update(cx, |this, cx| this.go(Section::Settings, window, cx)).ok();
                                    state.dismiss(window, cx);
                                }))
                        }),
                )
            })
    }
}

fn connection_badge(connection: &CamundaConnection, cx: &App) -> Div {
    h_flex().flex_1().min_w_0().gap_2().child(avatar(connection)).child(
        v_flex()
            .min_w_0()
            .child(div().truncate().text_sm().font_weight(FontWeight::SEMIBOLD).child(connection.name.clone()))
            .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(environment::label(&connection.environment))),
    )
}

fn connection_card(connection: &CamundaConnection, active: bool, cx: &App) -> Stateful<Div> {
    let host = connection.rest_url.split("://").nth(1).unwrap_or(&connection.rest_url).split('/').next().unwrap_or_default().to_string();
    v_flex()
        .id(SharedString::from(format!("connection-{}", connection.id)))
        .gap_2()
        .p_3()
        .min_w_0()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(if active { cx.theme().foreground } else { cx.theme().border })
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().secondary))
        .child(
            h_flex()
                .gap_2()
                .child(avatar(connection))
                .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight::SEMIBOLD).child(connection.name.clone()))
                .when(active, |this| this.child(Icon::new(Lucide::Check).small())),
        )
        .child(
            h_flex()
                .gap_2()
                .child(environment::tag(&connection.environment))
                .child(div().flex_1().min_w_0().truncate().text_xs().text_color(cx.theme().muted_foreground).child(host)),
        )
}

fn avatar(connection: &CamundaConnection) -> Avatar {
    Avatar::new().name(connection.name.clone()).small()
}
