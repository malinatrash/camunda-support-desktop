use gpui_kit::*;

use crate::platform::services::Services;

pub const LANGUAGES: [(&str, &str); 2] = [("en", "English"), ("ru", "Русский")];

// Язык интерфейса: английский по умолчанию, русский из настроек; общий для приложения, ядра и GPUI Kit.
pub fn apply(language: &str) {
    let language = LANGUAGES.iter().find(|(code, _)| *code == language).map_or("en", |(code, _)| code);
    rust_i18n::set_locale(language);
}

pub fn current() -> SharedString {
    SharedString::from(rust_i18n::locale().to_string())
}

pub fn set(language: &str, cx: &mut App) {
    apply(language);
    cx.global::<Services>().storage.save_language(language);
    cx.refresh_windows();
}
