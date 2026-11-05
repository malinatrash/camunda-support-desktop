use std::borrow::Cow;

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

pub const SANS: &str = "GitLab Sans";
pub const MONO: &str = "GitLab Mono";

const FONTS: [&[u8]; 8] = [
    include_bytes!("../assets/fonts/GitLabSans-Regular.ttf"),
    include_bytes!("../assets/fonts/GitLabSans-Medium.ttf"),
    include_bytes!("../assets/fonts/GitLabSans-Semibold.ttf"),
    include_bytes!("../assets/fonts/GitLabSans-Bold.ttf"),
    include_bytes!("../assets/fonts/GitLabMono-Regular.ttf"),
    include_bytes!("../assets/fonts/GitLabMono-Medium.ttf"),
    include_bytes!("../assets/fonts/GitLabMono-Semibold.ttf"),
    include_bytes!("../assets/fonts/GitLabMono-Bold.ttf"),
];

pub fn load_fonts(cx: &App) {
    if let Err(error) = cx.text_system().add_fonts(FONTS.iter().map(|font| Cow::Borrowed(*font)).collect()) {
        eprintln!("шрифты GitLab не загрузились: {error}");
    }
}

struct Palette {
    background: u32,
    surface: u32,
    subtle: u32,
    hover: u32,
    border: u32,
    border_strong: u32,
    ink: u32,
    ink_muted: u32,
    accent: u32,
    accent_hover: u32,
    accent_soft: u32,
    success: u32,
    warning: u32,
    danger: u32,
}

// Палитра Pajamas (дизайн-система GitLab), те же значения, что в веб-версии.
const LIGHT: Palette = Palette {
    background: 0xffffff,
    surface: 0xffffff,
    subtle: 0xfbfafd,
    hover: 0xf0f0f2,
    border: 0xdcdcde,
    border_strong: 0xbfbfc3,
    ink: 0x3a383f,
    ink_muted: 0x626168,
    accent: 0x1f75cb,
    accent_hover: 0x1068bf,
    accent_soft: 0xe9f3fc,
    success: 0x108548,
    warning: 0xab6100,
    danger: 0xdd2b0e,
};

const DARK: Palette = Palette {
    background: 0x18171d,
    surface: 0x18171d,
    subtle: 0x1f1e24,
    hover: 0x232128,
    border: 0x3a383f,
    border_strong: 0x535158,
    ink: 0xececef,
    ink_muted: 0xbfbfc3,
    accent: 0x428fdc,
    accent_hover: 0x63a6e9,
    accent_soft: 0x0e2a47,
    success: 0x2da160,
    warning: 0xc17d10,
    danger: 0xec5941,
};

pub fn apply(dark: bool, window: &mut Window, cx: &mut App) {
    Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, Some(window), cx);
    let p = if dark { &DARK } else { &LIGHT };
    Theme::update(cx, |theme| paint(theme, p, dark));
    window.refresh();
}

fn paint(theme: &mut Theme, p: &Palette, dark: bool) {
    let c = |value: u32| -> Hsla { rgb(value).into() };
    theme.font_family = SANS.into();
    theme.mono_font_family = MONO.into();
    theme.font_size = px(14.);
    theme.radius = px(4.);
    theme.radius_lg = px(8.);

    let colors = &mut theme.colors;
    colors.background = c(p.background);
    colors.foreground = c(p.ink);
    colors.border = c(p.border);
    colors.input = c(p.border_strong);
    colors.ring = c(p.accent);
    colors.muted = c(p.subtle);
    colors.muted_foreground = c(p.ink_muted);
    colors.popover = c(p.surface);
    colors.popover_foreground = c(p.ink);
    colors.accent = c(p.hover);
    colors.accent_foreground = c(p.ink);
    colors.primary = c(p.accent);
    colors.primary_hover = c(p.accent_hover);
    colors.primary_active = c(p.accent_hover);
    colors.primary_foreground = c(0xffffff);
    colors.button_primary = c(p.accent);
    colors.button_primary_hover = c(p.accent_hover);
    colors.button_primary_active = c(p.accent_hover);
    colors.button_primary_foreground = c(0xffffff);
    colors.button_danger = c(p.danger);
    colors.button_danger_hover = c(p.danger).opacity(0.9);
    colors.button_danger_active = c(p.danger).opacity(0.85);
    colors.button_danger_foreground = c(0xffffff);
    colors.button_secondary = c(if dark { 0x333238 } else { 0xececef });
    colors.button_secondary_hover = c(if dark { 0x434248 } else { 0xdcdcde });
    colors.button_secondary_active = c(if dark { 0x434248 } else { 0xdcdcde });
    colors.button_secondary_foreground = c(p.ink);
    colors.secondary = c(if dark { 0x333238 } else { 0xececef });
    colors.secondary_hover = c(if dark { 0x434248 } else { 0xdcdcde });
    colors.secondary_active = c(if dark { 0x434248 } else { 0xdcdcde });
    colors.secondary_foreground = c(p.ink);
    colors.link = c(p.accent);
    colors.link_hover = c(p.accent_hover);
    colors.link_active = c(p.accent_hover);
    colors.selection = c(p.accent).opacity(0.25);
    colors.caret = c(p.ink);
    colors.success = c(p.success);
    colors.warning = c(p.warning);
    colors.danger = c(p.danger);
    colors.info = c(p.accent);
    colors.blue = c(p.accent);
    colors.title_bar = c(p.subtle);
    colors.title_bar_border = c(p.border);
    colors.table = c(p.surface);
    colors.table_head = c(p.subtle);
    colors.table_head_foreground = c(p.ink_muted);
    colors.table_hover = c(p.hover);
    colors.table_active = c(p.accent_soft);
    colors.table_active_border = c(p.accent);
    colors.table_even = c(p.subtle);
    colors.table_row_border = c(p.border);
    colors.list = c(p.surface);
    colors.list_head = c(p.subtle);
    colors.list_hover = c(p.hover);
    colors.list_active = c(p.accent_soft);
    colors.list_active_border = c(p.accent);
    colors.tab = c(p.surface);
    colors.tab_bar = c(p.surface);
    colors.tab_active = c(p.surface);
    colors.tab_active_foreground = c(p.ink);
    colors.tab_foreground = c(p.ink_muted);
    colors.scrollbar_thumb = c(p.border_strong);
}
