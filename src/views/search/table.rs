use camunda_core::models::ProcessInstanceSummary;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::component::table::{Column, TableDelegate, TableState};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use crate::model;
use crate::platform::theme::mono;

pub struct ResultTable {
    pub(super) rows: Vec<ProcessInstanceSummary>,
    columns: Vec<Column>,
}

impl Default for ResultTable {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            columns: vec![
                Column::new("state", "Состояние").width(150.),
                Column::new("business_key", "Бизнес-ключ").width(240.),
                Column::new("process", "Процесс").width(240.),
                Column::new("id", "ID заявки").width(320.),
                Column::new("start", "Старт").width(160.),
            ],
        }
    }
}

impl TableDelegate for ResultTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let instance = &self.rows[row_ix];
        let muted = cx.theme().muted_foreground;
        match self.columns[col_ix].key.as_ref() {
            "state" => model::state_tag(instance.suspended, instance.ended).into_any_element(),
            "business_key" => {
                let key =
                    div().truncate().font_weight(FontWeight::MEDIUM).child(instance.business_key.clone().unwrap_or_else(|| "—".into()));
                instance_card("key", instance, key).into_any_element()
            }
            "process" => v_flex()
                .min_w_0()
                .child(div().truncate().child(instance.definition_name.clone().unwrap_or_else(|| instance.definition_key.clone())))
                .child(div().truncate().text_xs().font_family(mono(cx)).text_color(muted).child(instance.definition_id.clone()))
                .into_any_element(),
            "id" => instance_card("id", instance, div().truncate().text_sm().font_family(mono(cx)).child(instance.id.clone()))
                .into_any_element(),
            _ => div()
                .text_sm()
                .font_family(mono(cx))
                .text_color(muted)
                .child(model::short_time(instance.start_time.as_deref()))
                .into_any_element(),
        }
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<SharedString> {
        let instance = self.rows.get(row_ix)?;
        match self.columns[col_ix].key.as_ref() {
            "business_key" => instance.business_key.clone().map(Into::into),
            "id" => Some(instance.id.clone().into()),
            _ => None,
        }
    }
}

// Карточка заявки при наведении на бизнес-ключ или ID: процесс, версия, старт и состояние.
fn instance_card(cell: &str, instance: &ProcessInstanceSummary, trigger: impl IntoElement + 'static) -> HoverCard {
    let details = DescriptionList::new()
        .columns(1)
        .bordered(false)
        .item("Бизнес-ключ", instance.business_key.clone().unwrap_or_else(|| "—".into()), 1)
        .item("ID заявки", instance.id.clone(), 1)
        .item("Процесс", instance.definition_name.clone().unwrap_or_else(|| instance.definition_key.clone()), 1)
        .item("Версия", instance.definition_id.clone(), 1)
        .item("Старт", model::short_time(instance.start_time.as_deref()), 1)
        .item("Состояние", model::state_tag(instance.suspended, instance.ended).into_any_element(), 1);
    HoverCard::new(ElementId::Name(format!("instance-{cell}-{}", instance.id).into()))
        .anchor(Anchor::TopLeft)
        .trigger_style(StyleRefinement::default().w_full())
        .trigger(trigger)
        .child(div().w(px(460.)).child(details))
}
