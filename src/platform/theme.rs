use camunda_core::models::AppThemeMode;
use gpui_kit::component::{ActiveTheme, Theme, ThemeMode};
use gpui_kit::*;

use crate::platform::services::Services;

// Стандартные темы GPUI Kit: Default Dark по умолчанию, Default Light по выбору.
pub fn apply(dark: bool, window: &mut Window, cx: &mut App) {
    Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, Some(window), cx);
    translucent(cx);
}

pub fn toggle(window: &mut Window, cx: &mut App) {
    let dark = !cx.theme().is_dark();
    apply(dark, window, cx);
    remember(dark, cx);
}

// Переключение из настроек: окно не нужно, Theme::change обновляет все окна.
pub fn set_dark(dark: bool, cx: &mut App) {
    Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, None, cx);
    translucent(cx);
    remember(dark, cx);
}

fn remember(dark: bool, cx: &mut App) {
    cx.global::<Services>().storage.save_theme(if dark { AppThemeMode::Dark } else { AppThemeMode::Light });
}

pub fn mono(cx: &App) -> SharedString {
    cx.theme().mono_font_family.clone()
}

// Содержимое GroupBox без внутренних отступов: строки списков сами задают поля.
pub fn panel() -> StyleRefinement {
    StyleRefinement::default().p_0().gap_0().min_h_0().overflow_hidden()
}

// На macOS окно размыто: корень прозрачный, боковая панель полупрозрачная, содержимое на плотной подложке.
fn translucent(cx: &mut App) {
    if cfg!(target_os = "macos") {
        Theme::update(cx, |theme| {
            theme.colors.background.a = 0.;
            theme.colors.sidebar.a = if theme.is_dark() { 0.55 } else { 0.7 };
        });
    }
}

pub fn surface(cx: &App) -> Hsla {
    Hsla { a: 1., ..cx.theme().background }
}
