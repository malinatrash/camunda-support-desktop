use camunda_core::models::ProcessInstanceSummary;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::hover_card::HoverCard;
use gpui_kit::component::table::{Column, TableDelegate, TableState};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use crate::model;
use crate::platform::theme::mono;

// Колонки: ключ, ключ перевода заголовка и ширина; заголовок берётся при каждом refresh таблицы.
const COLUMNS: [(&str, &str, f32); 5] = [
    ("state", "search.column.state", 150.),
    ("business_key", "search.column.business_key", 240.),
    ("process", "search.column.process", 240.),
    ("id", "search.column.id", 320.),
    ("start", "search.column.start", 160.),
];

#[derive(Default)]
pub struct ResultTable {
    pub(super) rows: Vec<ProcessInstanceSummary>,
}

impl TableDelegate for ResultTable {
    fn columns_count(&self, _: &App) -> usize {
        COLUMNS.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        let (key, title, width) = COLUMNS[col_ix];
        Column::new(key, tr!(title)).width(width)
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let instance = &self.rows[row_ix];
        let muted = cx.theme().muted_foreground;
        match COLUMNS[col_ix].0 {
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
        match COLUMNS[col_ix].0 {
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
        .item(tr!("search.column.business_key"), instance.business_key.clone().unwrap_or_else(|| "—".into()), 1)
        .item(tr!("search.column.id"), instance.id.clone(), 1)
        .item(tr!("search.column.process"), instance.definition_name.clone().unwrap_or_else(|| instance.definition_key.clone()), 1)
        .item(tr!("search.column.version"), instance.definition_id.clone(), 1)
        .item(tr!("search.column.start"), model::short_time(instance.start_time.as_deref()), 1)
        .item(tr!("search.column.state"), model::state_tag(instance.suspended, instance.ended).into_any_element(), 1);
    HoverCard::new(ElementId::Name(format!("instance-{cell}-{}", instance.id).into()))
        .anchor(Anchor::TopLeft)
        .trigger_style(StyleRefinement::default().w_full())
        .trigger(trigger)
        .child(div().w(px(460.)).child(details))
}
