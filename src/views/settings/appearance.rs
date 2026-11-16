use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::setting::{SettingField, SettingGroup, SettingItem, SettingPage};

use crate::platform::theme;

pub fn page() -> SettingPage {
    SettingPage::new("Вид").icon(Lucide::Palette).group(
        SettingGroup::new().title("Тема").item(
            SettingItem::new("Тёмная тема", SettingField::switch(|cx| cx.theme().is_dark(), theme::set_dark))
                .description("Default Dark из GPUI Kit; выключите, чтобы перейти на Default Light."),
        ),
    )
}
