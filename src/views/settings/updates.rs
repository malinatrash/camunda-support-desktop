use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::*;

use crate::updates::{Status, Updater, VERSION};

pub fn page() -> SettingPage {
    SettingPage::new("Обновления").icon(Lucide::Download).group(
        SettingGroup::new()
            .title("Автообновление")
            .description("Новая версия скачивается и ставится сама, остаётся перезапустить приложение.")
            .item(SettingItem::render(|_, _, cx| status_row(cx))),
    )
}

fn status_row(cx: &mut App) -> Div {
    let updater = Updater::global(cx).read(cx);
    let status = updater.status();
    let description = status.describe(updater.progress().as_ref());
    let color = if matches!(status, Status::Failed(_)) { cx.theme().danger } else { cx.theme().muted_foreground };
    let action = match status {
        Status::Ready { .. } => {
            Some(Button::new("restart").label("Перезапустить").primary().small().on_click(|_, _, cx| Updater::restart(cx)))
        }
        status if status.is_busy() => None,
        _ => Some(Button::new("check-update").label("Проверить сейчас").small().on_click(|_, _, cx| {
            Updater::global(cx).update(cx, |updater, cx| updater.check(cx));
        })),
    };
    h_flex()
        .w_full()
        .gap_3()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(div().font_weight(FontWeight::MEDIUM).child(format!("Версия {VERSION}")))
                .child(div().text_sm().text_color(color).child(description)),
        )
        .children(action)
}
