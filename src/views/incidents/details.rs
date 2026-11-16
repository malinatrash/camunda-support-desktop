use camunda_core::models::ProcessIncident;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::*;

use super::load::definition_label;
use crate::model;

// Карточка при наведении: полное сообщение, кубик, процесс, заявка и время инцидента.
pub(super) fn incident_card(id: impl Into<SharedString>, incident: &ProcessIncident, trigger: impl IntoElement + 'static) -> HoverCard {
    let dash = || "—".to_string();
    let details = DescriptionList::new()
        .columns(1)
        .bordered(false)
        .item("Сообщение", incident.message.clone().unwrap_or_else(|| "Camunda не вернула сообщение ошибки".into()), 1)
        .item("Тип", model::incident_type_label(&incident.incident_type).to_string(), 1)
        .item("Кубик", incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_else(dash), 1)
        .item("Процесс", incident.process_definition_id.as_deref().map(definition_label).unwrap_or_else(dash), 1)
        .item("Заявка", incident.process_instance_id.clone().unwrap_or_else(dash), 1)
        .item("Время", model::short_time(incident.timestamp.as_deref()), 1);
    HoverCard::new(ElementId::Name(id.into()))
        .anchor(Anchor::TopLeft)
        .trigger_style(StyleRefinement::default().w_full())
        .trigger(trigger)
        .child(div().w(px(460.)).child(details))
}
