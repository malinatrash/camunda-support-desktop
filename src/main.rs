#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod bpmn;
mod model;
mod platform;
mod updates;
mod views;

use camunda_core::migration;
use camunda_core::models::AppThemeMode;
use futures::channel::mpsc;
use gpui_kit::component::TitleBar;
use gpui_kit::*;

use app::AppView;
use platform::services::Services;

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
        let services = Services::new();
        migration::migrate_legacy_preferences(&services.storage);
        let dark = services.storage.load_theme() != Some(AppThemeMode::Light);
        cx.set_global(services);
        updates::Updater::init(cx);
        app::bind_keys(cx);
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
            window_background: if cfg!(target_os = "macos") {
                WindowBackgroundAppearance::Blurred
            } else {
                WindowBackgroundAppearance::Opaque
            },
            ..TitleBar::window_options()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            window.set_window_title("Поддержка Camunda");
            platform::theme::apply(dark, window, cx);
            cx.new(|cx| AppView::new(received, window, cx))
        })
        .expect("окно не открылось");
        cx.activate(true);
        drop(links);
    });
}
