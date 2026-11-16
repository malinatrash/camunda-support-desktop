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

// Форма по числу: ключи `<key>.one/.few/.many` для ru и `<key>.one/.other` для en, в тексте `%{count}`.
#[allow(dead_code)]
pub fn plural(key: &str, count: i64) -> SharedString {
    let locale = rust_i18n::locale();
    let key = format!("{key}.{}", plural_form(&locale, count));
    SharedString::from(rust_i18n::t!(&key, count = count).into_owned())
}

fn plural_form(locale: &str, count: i64) -> &'static str {
    let n = count.unsigned_abs();
    match locale {
        "ru" if n % 10 == 1 && n % 100 != 11 => "one",
        "ru" if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) => "few",
        "ru" => "many",
        _ if n == 1 => "one",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::plural_form;

    #[test]
    fn plural_forms_follow_language_rules() {
        let forms = |locale| [1, 2, 5, 11, 21, 22, 112].map(|n| plural_form(locale, n));
        assert_eq!(forms("ru"), ["one", "few", "many", "many", "one", "few", "many"]);
        assert_eq!(forms("en"), ["one", "other", "other", "other", "other", "other", "other"]);
    }

    #[test]
    fn russian_keeps_original_text() {
        assert_eq!(rust_i18n::t!("model.duration.hours_minutes", locale = "ru", hours = 2, minutes = 5), "2 ч. 5 мин.");
        assert_eq!(rust_i18n::t!("sidebar.version", locale = "ru", version = "1.0"), "версия 1.0");
    }
}
