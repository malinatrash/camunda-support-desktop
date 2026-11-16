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

// Форма множественного числа: ru — one/few/many, en — one/other.
pub(super) fn plural(key: &str, count: usize) -> SharedString {
    let form = if &*rust_i18n::locale() == "ru" {
        match (count % 10, count % 100) {
            (1, n) if n != 11 => "one",
            (2..=4, n) if !(12..=14).contains(&n) => "few",
            _ => "many",
        }
    } else if count == 1 {
        "one"
    } else {
        "other"
    };
    tr!(&format!("{key}.{form}"), count = count)
}

#[cfg(test)]
mod tests {
    use super::plural;

    #[test]
    fn picks_english_plural_forms() {
        assert_eq!(plural("instance.timeline.events", 1).as_ref(), "1 event · History API");
        assert_eq!(plural("instance.timeline.events", 5).as_ref(), "5 events · History API");
    }
}
