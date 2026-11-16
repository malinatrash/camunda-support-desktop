use std::collections::HashSet;

use camunda_core::models::ProcessIncident;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::table::{Column, TableDelegate, TableState};
use gpui_kit::*;

use super::details::incident_card;
use super::load::definition_label;
use crate::model;
use crate::platform::theme::mono;

pub struct IncidentTable {
    pub(super) rows: Vec<ProcessIncident>,
    pub(super) selected: HashSet<String>,
    columns: Vec<Column>,
}

impl Default for IncidentTable {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            selected: HashSet::new(),
            columns: vec![
                Column::new("pick", "").width(44.).resizable(false),
                Column::new("message", "Сообщение").width(420.),
                Column::new("instance", "Заявка").width(300.),
                Column::new("type", "Тип").width(160.),
                Column::new("activity", "Кубик").width(200.),
                Column::new("process", "Процесс").width(180.),
                Column::new("time", "Время").width(150.),
            ],
        }
    }
}

impl TableDelegate for IncidentTable {
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
        let incident = &self.rows[row_ix];
        let muted = cx.theme().muted_foreground;
        let cell = div().truncate().text_sm();
        match self.columns[col_ix].key.as_ref() {
            "pick" => {
                let id = incident.id.clone();
                Checkbox::new(SharedString::from(format!("pick-{}", incident.id)))
                    .checked(self.selected.contains(&incident.id))
                    .on_click(cx.listener(move |table, checked: &bool, _, cx| {
                        let selected = &mut table.delegate_mut().selected;
                        if *checked {
                            selected.insert(id.clone());
                        } else {
                            selected.remove(&id);
                        }
                        cx.notify();
                    }))
                    .into_any_element()
            }
            "message" => {
                let message = cell.child(incident.message.clone().unwrap_or_else(|| "Camunda не вернула сообщение ошибки".into()));
                incident_card(format!("card-{}", incident.id), incident, message).into_any_element()
            }
            "instance" => {
                cell.font_family(mono(cx)).child(incident.process_instance_id.clone().unwrap_or_else(|| "—".into())).into_any_element()
            }
            "type" => cell.text_color(muted).child(model::incident_type_label(&incident.incident_type).to_string()).into_any_element(),
            "activity" => cell.font_family(mono(cx)).child(activity(incident).unwrap_or_else(|| "—".into())).into_any_element(),
            "process" => {
                cell.child(incident.process_definition_id.as_deref().map(definition_label).unwrap_or_else(|| "—".into())).into_any_element()
            }
            _ => cell.font_family(mono(cx)).text_color(muted).child(model::short_time(incident.timestamp.as_deref())).into_any_element(),
        }
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<SharedString> {
        let incident = self.rows.get(row_ix)?;
        match self.columns[col_ix].key.as_ref() {
            "message" => incident.message.clone().map(Into::into),
            "instance" => incident.process_instance_id.clone().map(Into::into),
            "activity" => activity(incident).map(Into::into),
            _ => None,
        }
    }
}

fn activity(incident: &ProcessIncident) -> Option<String> {
    incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone())
}
