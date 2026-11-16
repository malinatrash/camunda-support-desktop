use camunda_core::models::AppThemeMode;
use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
use gpui_kit::*;

use crate::platform::services::Services;

// Стандартные темы GPUI Kit: Default Dark по умолчанию, Default Light по выбору.
pub fn apply(dark: bool, window: &mut Window, cx: &mut App) {
    Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, Some(window), cx);
}

pub fn toggle(window: &mut Window, cx: &mut App) {
    let dark = !cx.theme().is_dark();
    apply(dark, window, cx);
    cx.global::<Services>().storage.save_theme(if dark { AppThemeMode::Dark } else { AppThemeMode::Light });
}

pub fn mono(cx: &App) -> SharedString {
    cx.theme().mono_font_family.clone()
}

// Содержимое GroupBox без внутренних отступов: строки списков сами задают поля.
pub fn panel() -> StyleRefinement {
    StyleRefinement::default().p_0().gap_0().min_h_0().overflow_hidden()
}
