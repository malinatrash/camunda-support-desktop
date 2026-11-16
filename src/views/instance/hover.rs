use gpui_kit::component::description_list::{DescriptionItem, DescriptionList};
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::component::{ActiveTheme, Sizable, v_flex};
use gpui_kit::*;

// Карточка при наведении со списком «поле — значение».
pub(super) fn fields_card(id: impl Into<ElementId>, trigger: impl IntoElement + 'static, fields: Vec<(&'static str, String)>) -> HoverCard {
    HoverCard::new(id).trigger_style(StyleRefinement::default().min_w_0()).trigger(trigger).content(move |_, _, _| {
        let items = fields.iter().map(|(label, value)| DescriptionItem::new(*label).value(value.clone()));
        div().w(px(440.)).child(DescriptionList::new().columns(1).bordered(false).small().children(items))
    })
}

// Карточка при наведении с полным текстом: сообщение, стек или значение.
pub(super) fn text_card(id: impl Into<ElementId>, trigger: impl IntoElement + 'static, title: String, text: String) -> HoverCard {
    HoverCard::new(id).trigger_style(StyleRefinement::default().min_w_0()).trigger(trigger).content(move |_, _, cx| {
        v_flex().w(px(560.)).gap_2().child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title.clone())).child(
            div()
                .id("hover-text")
                .max_h(px(360.))
                .overflow_y_scroll()
                .text_xs()
                .font_family(crate::platform::theme::mono(cx))
                .text_color(cx.theme().foreground)
                .child(text.clone()),
        )
    })
}
