#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod bpmn;
mod definition;
mod incidents;
mod instance;
mod model;
mod operations;
mod overview;
mod palette;
mod search;
mod services;
mod settings;
mod teleport;
mod theme;
mod ui;
mod variables;

use camunda_core::migration;
use camunda_core::models::AppThemeMode;
use futures::channel::mpsc;
use gpui_kit::component::TitleBar;
use gpui_kit::*;

use app::{AppView, OpenIncidents, OpenOverview, OpenSearch, OpenSettings, TogglePalette};
use services::Services;

actions!(camunda_support, [Quit]);

fn main() {
    let (links, received) = mpsc::unbounded::<String>();
    for argument in std::env::args().skip(1).filter(|argument| argument.starts_with("camunda-support:")) {
        links.unbounded_send(argument).ok();
    }
    let application = gpui_kit::application().with_assets(gpui_kit::assets::Assets);
    let url_sender = links.clone();
    application.on_open_urls(move |urls| {
        for url in urls {
            url_sender.unbounded_send(url).ok();
        }
    });
    application.run(move |cx| {
        gpui_kit::init(cx);
        theme::load_fonts(cx);
        let services = Services::new();
        migration::migrate_legacy_preferences(&services.storage);
        let dark = services.storage.load_theme() == Some(AppThemeMode::Dark);
        cx.set_global(services);
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-k", TogglePalette, None),
            KeyBinding::new("ctrl-k", TogglePalette, None),
            KeyBinding::new("cmd-1", OpenOverview, None),
            KeyBinding::new("cmd-2", OpenSearch, None),
            KeyBinding::new("cmd-3", OpenIncidents, None),
            KeyBinding::new("cmd-,", OpenSettings, None),
        ]);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1440.), px(900.)), cx)),
            window_min_size: Some(size(px(1080.), px(680.))),
            app_id: Some("camunda-support".into()),
            ..TitleBar::window_options()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            window.set_window_title("Поддержка Camunda");
            theme::apply(dark, window, cx);
            cx.new(|cx| AppView::new(received, window, cx))
        })
        .expect("окно не открылось");
        cx.activate(true);
        drop(links);
    });
}
