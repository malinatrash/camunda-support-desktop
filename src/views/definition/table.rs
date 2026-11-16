use camunda_core::models::RuntimeInstanceListItem;
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyTitle};
use gpui_kit::component::table::{Column, TableDelegate, TableState};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::*;

use crate::model;

// Колонки: ключ, ключ перевода заголовка и ширина; заголовок берётся при каждом refresh таблицы.
const COLUMNS: [(&str, &str, f32); 7] = [
    ("state", "definition.column.state", 140.),
    ("business_key", "definition.column.business_key", 220.),
    ("id", "definition.column.id", 320.),
    ("start", "definition.column.start", 160.),
    ("version", "definition.column.version", 80.),
    ("incidents", "definition.column.incidents", 100.),
    ("tenant", "definition.column.tenant", 140.),
];

#[derive(Default)]
pub struct InstanceTable {
    pub(super) rows: Vec<RuntimeInstanceListItem>,
    pub(super) loading: bool,
}

impl TableDelegate for InstanceTable {
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

    fn loading(&self, _: &App) -> bool {
        self.loading && self.rows.is_empty()
    }

    fn render_empty(&mut self, _: &mut Window, _: &mut Context<TableState<Self>>) -> impl IntoElement {
        Empty::new().header(EmptyHeader::new().title(EmptyTitle::new().child(tr!("definition.empty"))))
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let item = &self.rows[row_ix];
        let instance = &item.instance;
        let muted = cx.theme().muted_foreground;
        match COLUMNS[col_ix].0 {
            "state" => model::state_tag(instance.suspended, instance.ended).into_any_element(),
            "business_key" => div().truncate().child(instance.business_key.clone().unwrap_or_else(|| "—".into())).into_any_element(),
            "id" => div().truncate().text_sm().font_family(crate::platform::theme::mono(cx)).child(instance.id.clone()).into_any_element(),
            "start" => div()
                .text_sm()
                .font_family(crate::platform::theme::mono(cx))
                .text_color(muted)
                .child(model::short_time(instance.start_time.as_deref()))
                .into_any_element(),
            "version" => {
                let version = instance.definition_id.split(':').nth(1).unwrap_or("?");
                div().text_sm().text_color(muted).child(format!("v{version}")).into_any_element()
            }
            "incidents" if item.incident_count > 0 => {
                Tag::danger().small().rounded_full().child(item.incident_count.to_string()).into_any_element()
            }
            "incidents" => div().text_color(muted).child("0").into_any_element(),
            _ => div()
                .truncate()
                .text_sm()
                .text_color(muted)
                .child(instance.tenant_id.clone().unwrap_or_else(|| "—".into()))
                .into_any_element(),
        }
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<SharedString> {
        let instance = &self.rows.get(row_ix)?.instance;
        match COLUMNS[col_ix].0 {
            "business_key" => instance.business_key.clone().map(Into::into),
            "id" => Some(instance.id.clone().into()),
            _ => None,
        }
    }
}
