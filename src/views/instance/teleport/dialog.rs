use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::{TeleportEvent, TeleportView};
use crate::views::instance::InstanceView;

impl InstanceView {
    // Открывает диалог телепорта, после успеха обновляет заявку.
    pub(in crate::views::instance) fn open_teleport(&mut self, target: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(details) = &self.details else { return };
        let connection = self.connection.clone();
        let teleport = cx.new(|cx| TeleportView::new(connection, details, target, window, cx));
        self._teleport_subscription = Some(cx.subscribe_in(&teleport, window, |this, _, event: &TeleportEvent, window, cx| {
            this.teleport = None;
            if window.has_active_dialog(cx) {
                window.close_dialog(cx);
            }
            if matches!(event, TeleportEvent::Done) {
                this.notice = Some("Телепорт выполнен, состояние заявки обновлено.".into());
                this.reload(window, cx);
            }
            cx.notify();
        }));
        self.teleport = Some(teleport.clone());
        window.open_dialog(cx, move |dialog, window, _| {
            dialog
                .title("Телепорт заявки")
                .w(window.viewport_size().width * 0.8)
                .max_w(px(1200.))
                .overlay_closable(false)
                .child(teleport.clone())
        });
    }

    pub(in crate::views::instance) fn can_teleport(&self) -> bool {
        self.details
            .as_ref()
            .is_some_and(|details| !details.instance.ended && details.active_activities.iter().any(|activity| activity.cancellable))
    }
}
