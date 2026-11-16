use camunda_core::models::ProcessInstanceDetails;
use gpui_kit::component::description_list::{DescriptionItem, DescriptionList};
use gpui_kit::component::v_flex;
use gpui_kit::*;

// Метаданные заявки списком «поле — значение» с прокруткой.
pub(super) fn render(details: &ProcessInstanceDetails, cx: &App) -> impl IntoElement {
    let items = details.metadata.iter().map(|field| {
        let value = div().font_family(crate::platform::theme::mono(cx)).child(field.value.clone()).into_any_element();
        DescriptionItem::new(field.label.clone()).value(value)
    });
    v_flex()
        .id("metadata")
        .flex_1()
        .min_h_0()
        .gap_3()
        .overflow_y_scroll()
        .child(div().font_weight(FontWeight::SEMIBOLD).child("Метаданные заявки"))
        .child(DescriptionList::new().columns(1).label_width(relative(0.3)).children(items))
}
