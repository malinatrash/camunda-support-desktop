use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::*;

#[derive(Clone)]
pub enum Navigate {
    Definition { id: String, all_versions: bool },
    Instance(String),
    Back,
}

#[derive(Clone, Copy)]
pub enum Tone {
    Danger,
    Warning,
    Success,
    Info,
    Neutral,
}

// Бейдж с цветным фоном в стиле GitLab.
pub fn pill(text: impl Into<SharedString>, tone: Tone, cx: &App) -> Div {
    let theme = cx.theme();
    let color = match tone {
        Tone::Danger => theme.danger,
        Tone::Warning => theme.warning,
        Tone::Success => theme.success,
        Tone::Info => theme.info,
        Tone::Neutral => theme.muted_foreground,
    };
    div()
        .flex_none()
        .px_2()
        .rounded_full()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .bg(color.opacity(0.16))
        .text_color(color)
        .child(text.into())
}

pub fn counter(value: usize, cx: &App) -> Div {
    div()
        .flex_none()
        .px_1p5()
        .rounded_full()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .bg(cx.theme().secondary)
        .text_color(cx.theme().muted_foreground)
        .child(value.to_string())
}

// Секция в рамке с серой шапкой.
pub fn section(head: impl IntoElement, cx: &App) -> Div {
    v_flex().min_w_0().min_h_0().border_1().border_color(cx.theme().border).rounded_md().overflow_hidden().bg(cx.theme().background).child(
        h_flex().flex_none().h(px(46.)).px_4().gap_3().bg(cx.theme().table_head).border_b_1().border_color(cx.theme().border).child(head),
    )
}

pub fn instance_state(suspended: bool, ended: bool, cx: &App) -> Div {
    if ended {
        pill("завершена", Tone::Neutral, cx)
    } else if suspended {
        pill("приостановлена", Tone::Warning, cx)
    } else {
        pill("в работе", Tone::Success, cx)
    }
}
