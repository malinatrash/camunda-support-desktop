use camunda_core::models::CamundaConnection;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::{ActiveTheme, Sizable, WindowExt as _, h_flex, v_flex};
use gpui_kit::*;

use super::SettingsView;
use crate::platform::environment;
use crate::platform::services::Services;

impl SettingsView {
    // Страница подключений: по пункту на подключение, добавление в диалоге.
    pub(super) fn connections_page(&self, cx: &mut Context<Self>) -> SettingPage {
        let view = cx.entity().downgrade();
        let add = view.clone();
        let rows = self.connections.iter().map(|connection| {
            let (connection, view) = (connection.clone(), view.clone());
            SettingItem::render(move |_, _, cx| connection_row(&connection, view.clone(), cx))
        });
        let empty = self.connections.is_empty().then(|| {
            SettingItem::render(|_, _, cx| {
                div().text_sm().text_color(cx.theme().muted_foreground).child("Подключений пока нет — добавьте первое.")
            })
        });
        SettingPage::new("Подключения").icon(Lucide::Server).default_open(true).group(
            SettingGroup::new()
                .title("Подключения к Camunda")
                .description("Адрес Cockpit строится из адреса REST автоматически.")
                .items(rows.chain(empty))
                .footer(move |_, _| {
                    let add = add.clone();
                    Button::new("add-connection").label("Добавить подключение").icon(Lucide::Plus).small().on_click(move |_, window, cx| {
                        add.update(cx, |this, cx| this.open_form(window, cx)).ok();
                    })
                }),
        )
    }

    fn delete_connection(&mut self, id: &str, cx: &mut Context<Self>) {
        cx.global::<Services>().storage.delete_connection(id);
        self.reload_connections(cx);
    }
}

fn connection_row(connection: &CamundaConnection, view: WeakEntity<SettingsView>, cx: &mut App) -> Div {
    let (id, name) = (connection.id.clone(), connection.name.clone());
    let engine = connection.engine_version.clone().map(|version| format!(" · Camunda {version}")).unwrap_or_default();
    h_flex()
        .w_full()
        .gap_3()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    h_flex()
                        .gap_2()
                        .child(div().font_weight(FontWeight::MEDIUM).child(connection.name.clone()))
                        .child(environment::tag(&connection.environment)),
                )
                .child(
                    div()
                        .truncate()
                        .text_xs()
                        .font_family(crate::platform::theme::mono(cx))
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{}{engine}", connection.rest_url)),
                ),
        )
        .child(Button::new(SharedString::from(format!("delete-{id}"))).icon(Lucide::Trash).ghost().small().tooltip("Удалить").on_click(
            move |_, window, cx| {
                let (id, view, name) = (id.clone(), view.clone(), name.clone());
                window.open_alert_dialog(cx, move |alert, _, _| {
                    let (id, view) = (id.clone(), view.clone());
                    alert
                        .title(format!("Удалить «{name}»?"))
                        .description("Подключение пропадёт из списка, его можно будет добавить заново.")
                        .show_cancel(true)
                        .on_ok(move |_, _, cx| {
                            view.update(cx, |this, cx| this.delete_connection(&id, cx)).ok();
                            true
                        })
                });
            },
        ))
}
