use camunda_core::deep_link;
use camunda_core::models::DeepLinkEvent;
use futures::StreamExt as _;
use futures::channel::mpsc::UnboundedReceiver;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::notification::Notification;
use gpui_kit::*;

use super::AppView;
use super::palette::Section;
use crate::model::Navigate;

// Ссылки camunda-support:// из аргументов запуска и от системы.
pub fn listen(links: UnboundedReceiver<String>, window: &mut Window, cx: &mut Context<AppView>) -> Task<()> {
    cx.spawn_in(window, async move |this, cx| {
        let mut links = links;
        while let Some(url) = links.next().await {
            if this.update_in(cx, |this, window, cx| this.open_link(&url, window, cx)).is_err() {
                break;
            }
        }
    })
}

impl AppView {
    // Открывает заявку в подключении с тем же доменом, что у ссылки.
    fn open_link(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        window.activate_window();
        match deep_link::parse_deep_link(url) {
            DeepLinkEvent::OpenProcessInstance { link } => {
                let Some(connection) = self.connections.iter().find(|c| deep_link::matches_origin(c, &link.origin)).cloned() else {
                    let message = tr!("shell.deep_link_no_connection", origin = link.origin);
                    window.push_notification(Notification::warning(message), cx);
                    return;
                };
                if self.connection.as_ref().map(|c| &c.id) != Some(&connection.id) {
                    self.select_connection(connection, window, cx);
                }
                self.section = Section::Overview;
                self.pages.clear();
                self.open_page(Navigate::Instance(link.process_instance_id), window, cx);
            }
            DeepLinkEvent::Invalid { message } => window.push_notification(Notification::error(message), cx),
        }
    }
}
