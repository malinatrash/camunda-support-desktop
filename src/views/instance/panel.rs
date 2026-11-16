use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::InputState;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::*;

// Рамка списка во всю высоту: заголовок сверху, содержимое прокручивает вызывающий.
pub(super) fn list_panel(head: impl IntoElement, cx: &App) -> Div {
    v_flex()
        .flex_1()
        .min_h_0()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .overflow_hidden()
        .child(h_flex().flex_none().px_4().py_3().gap_3().border_b_1().border_color(cx.theme().border).child(head))
}

pub(super) fn counted(title: SharedString, count: usize) -> impl IntoElement {
    h_flex()
        .w_full()
        .gap_2()
        .child(div().flex_1().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(Tag::secondary().small().rounded_full().child(count.to_string()))
}

pub(super) fn empty(text: SharedString) -> Empty {
    Empty::new().py_6().header(EmptyHeader::new().description(EmptyDescription::new().child(text)))
}

// Обновляет подсказку поля, если язык сменился после создания.
pub(super) fn sync_placeholder(input: &Entity<InputState>, text: SharedString, window: &mut Window, cx: &mut App) {
    if input.read(cx).presentation().placeholder() != &text {
        input.update(cx, |input, cx| input.set_placeholder(text, window, cx));
    }
}
