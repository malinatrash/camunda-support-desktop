use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::setting::{SettingField, SettingGroup, SettingItem, SettingPage};
use gpui_kit::*;

use crate::platform::{locale, theme};

pub fn page() -> SettingPage {
    let languages = locale::LANGUAGES.iter().map(|(code, name)| (SharedString::from(*code), SharedString::from(*name))).collect();
    SettingPage::new(tr!("settings.appearance.title"))
        .icon(Lucide::Palette)
        .group(
            SettingGroup::new().title(tr!("settings.appearance.theme")).item(
                SettingItem::new(tr!("settings.appearance.dark"), SettingField::switch(|cx| cx.theme().is_dark(), theme::set_dark))
                    .description(tr!("settings.appearance.dark_hint")),
            ),
        )
        .group(
            SettingGroup::new().title(tr!("settings.appearance.language")).item(
                SettingItem::new(
                    tr!("settings.appearance.language_label"),
                    SettingField::dropdown(languages, |_| locale::current(), |language, cx| locale::set(&language, cx)),
                )
                .description(tr!("settings.appearance.language_hint")),
            ),
        )
}
