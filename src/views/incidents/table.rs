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

// Колонки: ключ, ключ перевода заголовка и ширина; заголовок берётся при каждом refresh таблицы.
const COLUMNS: [(&str, &str, f32); 7] = [
    ("pick", "", 44.),
    ("message", "incidents.field.message", 420.),
    ("instance", "incidents.field.instance", 300.),
    ("type", "incidents.field.type", 160.),
    ("activity", "incidents.field.activity", 200.),
    ("process", "incidents.field.process", 180.),
    ("time", "incidents.field.time", 150.),
];

#[derive(Default)]
pub struct IncidentTable {
    pub(super) rows: Vec<ProcessIncident>,
    pub(super) selected: HashSet<String>,
}

impl TableDelegate for IncidentTable {
    fn columns_count(&self, _: &App) -> usize {
        COLUMNS.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        match COLUMNS[col_ix] {
            (key, "", width) => Column::new(key, "").width(width).resizable(false),
            (key, title, width) => Column::new(key, tr!(title)).width(width),
        }
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let incident = &self.rows[row_ix];
        let muted = cx.theme().muted_foreground;
        let cell = div().truncate().text_sm();
        match COLUMNS[col_ix].0 {
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
                let message = cell.child(incident.message.clone().unwrap_or_else(|| tr!("incidents.no_message").into()));
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
        match COLUMNS[col_ix].0 {
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
