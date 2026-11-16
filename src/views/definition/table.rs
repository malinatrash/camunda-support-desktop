use camunda_core::models::RuntimeInstanceListItem;
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyTitle};
use gpui_kit::component::table::{Column, TableDelegate, TableState};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::*;

use crate::model;

pub struct InstanceTable {
    pub(super) rows: Vec<RuntimeInstanceListItem>,
    pub(super) loading: bool,
    columns: Vec<Column>,
}

impl Default for InstanceTable {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            loading: false,
            columns: vec![
                Column::new("state", "Состояние").width(140.),
                Column::new("business_key", "Бизнес-ключ").width(220.),
                Column::new("id", "ID заявки").width(320.),
                Column::new("start", "Создана").width(160.),
                Column::new("version", "Версия").width(80.),
                Column::new("incidents", "Инциденты").width(100.),
                Column::new("tenant", "Тенант").width(140.),
            ],
        }
    }
}

impl TableDelegate for InstanceTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn loading(&self, _: &App) -> bool {
        self.loading && self.rows.is_empty()
    }

    fn render_empty(&mut self, _: &mut Window, _: &mut Context<TableState<Self>>) -> impl IntoElement {
        Empty::new().header(EmptyHeader::new().title(EmptyTitle::new().child("Заявок нет")))
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let item = &self.rows[row_ix];
        let instance = &item.instance;
        let muted = cx.theme().muted_foreground;
        match self.columns[col_ix].key.as_ref() {
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
        match self.columns[col_ix].key.as_ref() {
            "business_key" => instance.business_key.clone().map(Into::into),
            "id" => Some(instance.id.clone().into()),
            _ => None,
        }
    }
}
