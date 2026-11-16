use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

pub struct Palette {
    pub background: Hsla,
    pub grid: Hsla,
    pub stroke: Hsla,
    pub text: Hsla,
    pub active: Hsla,
    pub danger: Hsla,
    pub success: Hsla,
}

// Цвета схемы из текущей темы.
pub fn palette(cx: &App) -> Palette {
    let theme = cx.theme();
    Palette {
        background: theme.background,
        grid: theme.border.opacity(0.35),
        stroke: theme.foreground.opacity(0.75),
        text: theme.foreground,
        active: theme.blue,
        danger: theme.danger,
        success: theme.success,
    }
}
