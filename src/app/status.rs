use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::AppView;
use super::palette::Section;
use crate::platform::{environment, locale, motion, theme};
use crate::updates::{Updater, VERSION};

impl AppView {
    // Нижняя строка: живое подключение и движок слева, язык, тема и версия справа.
    pub(super) fn render_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let left = match &self.connection {
            Some(connection) => {
                let host = connection.rest_url.split("://").nth(1).unwrap_or(&connection.rest_url).split('/').next().unwrap_or_default().to_string();
                h_flex()
                    .gap_2()
                    .min_w_0()
                    .child(motion::pulse("live-dot", div().size(px(7.)).rounded_full().bg(cx.theme().success)))
                    .child(div().text_color(cx.theme().foreground).child(tr!("shell.status.live")))
                    .child(div().truncate().child(host))
                    .child(environment::tag(&connection.environment).xsmall())
                    .when_some(connection.engine_version.clone(), |this, version| this.child(tr!("shell.status.engine", version = version)))
            }
            None => h_flex().gap_2().child(div().size(px(7.)).rounded_full().bg(muted)).child(tr!("shell.status.offline")),
        };
        let updating = Updater::global(cx).read(cx).status().is_busy();
        let dark = cx.theme().is_dark();
        StatusBar::new()
            .px_3()
            .py_1()
            .text_xs()
            .text_color(muted)
            .border_t_1()
            .border_color(cx.theme().border)
            .left(left)
            .right(
                h_flex()
                    .gap_1()
                    .when(updating, |this| this.child(Icon::new(Lucide::Download).xsmall()).child(tr!("shell.status.updating")))
                    .child(
                        Button::new("status-language")
                            .ghost()
                            .xsmall()
                            .label(locale::current().to_uppercase())
                            .on_click(|_, _, cx| locale::set(if locale::current() == "ru" { "en" } else { "ru" }, cx)),
                    )
                    .child(
                        Button::new("status-theme")
                            .ghost()
                            .xsmall()
                            .icon(if dark { Lucide::Sun } else { Lucide::Moon })
                            .on_click(|_, window, cx| theme::toggle(window, cx)),
                    )
                    .child(div().px_1().child(format!("v{VERSION}"))),
            )
    }

    // Первый запуск без подключений: приглашение добавить подключение.
    pub(super) fn render_welcome(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().items_center().justify_center().child(
            Empty::new()
                .header(
                    EmptyHeader::new()
                        .media(EmptyMedia::new().child(Icon::new(Lucide::Workflow).large()))
                        .title(EmptyTitle::new().child(tr!("shell.welcome.title")))
                        .description(EmptyDescription::new().child(tr!("shell.welcome.description"))),
                )
                .content(
                    EmptyContent::new().child(
                        Button::new("welcome-add")
                            .primary()
                            .icon(Lucide::Plus)
                            .label(tr!("shell.welcome.add"))
                            .on_click(cx.listener(|this, _, window, cx| this.go(Section::Settings, window, cx))),
                    ),
                ),
        )
    }
}
