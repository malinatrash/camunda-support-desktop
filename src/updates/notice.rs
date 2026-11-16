use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::notification::Notification;
use gpui_kit::*;

use super::{Status, Updater};

// Когда новая версия встала, один раз показывает уведомление с кнопкой перезапуска.
pub fn announce<V: 'static>(window: &mut Window, cx: &mut Context<V>) -> Subscription {
    cx.observe_in(&Updater::global(cx), window, |_, updater, window, cx| {
        let version = match updater.read(cx).status() {
            Status::Ready { version, .. } if !updater.read(cx).announced => version.clone(),
            _ => return,
        };
        updater.update(cx, |updater, _| updater.announced = true);
        let note = Notification::success(tr!("updates.installed_notice", version = version))
            .title(tr!("updates.ready_title"))
            .action(|_, _, _| Button::new("restart").primary().label(tr!("updates.restart")).on_click(|_, _, cx| Updater::restart(cx)));
        window.push_notification(note, cx);
    })
}
