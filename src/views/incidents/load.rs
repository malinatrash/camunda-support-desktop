use camunda_core::models::DashboardDateFilter;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::select::SelectItem;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::{IndexPath, v_flex};
use gpui_kit::*;

use super::IncidentsView;
use crate::platform::services;

#[derive(Clone)]
pub struct DefinitionOption {
    label: SharedString,
    value: Option<String>,
}

impl DefinitionOption {
    pub(super) fn all() -> Self {
        Self { label: tr!("incidents.all_processes"), value: None }
    }
}

impl SelectItem for DefinitionOption {
    type Value = Option<String>;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Option<String> {
        &self.value
    }
}

impl IncidentsView {
    pub(super) fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let filter = DashboardDateFilter { preset: self.preset, ..Default::default() };
        let definition = self.selected_definition(cx);
        let request =
            services::request(cx, move |api| async move { api.load_incidents(&connection, definition.as_deref(), &filter, None).await });
        self.error = None;
        self.load = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(incidents) => {
                        this.incidents = incidents;
                        this.sync_definitions(window, cx);
                        this.sync_table(cx);
                    }
                    Err(error) => this.error = Some(error),
                }
                this.load = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn selected_definition(&self, cx: &App) -> Option<String> {
        self.definition.read(cx).selected_value().cloned().flatten()
    }

    // Пополняет список версий процессов из загруженных инцидентов, не сбрасывая выбор.
    fn sync_definitions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.known_definitions.len();
        for id in self.incidents.iter().filter_map(|incident| incident.process_definition_id.clone()) {
            if !self.known_definitions.contains(&id) {
                self.known_definitions.push(id);
            }
        }
        if self.known_definitions.len() == before {
            return;
        }
        self.known_definitions.sort();
        let selected = self.selected_definition(cx);
        let options = self.definition_options();
        let index = selected.and_then(|id| self.known_definitions.iter().position(|known| *known == id)).map_or(0, |position| position + 1);
        self.definition.update(cx, |state, cx| {
            state.set_items(options, window, cx);
            state.set_selected_index(Some(IndexPath::default().row(index)), window, cx);
        });
    }

    // «Все процессы» и известные версии процессов с подписями на текущем языке.
    pub(super) fn definition_options(&self) -> Vec<DefinitionOption> {
        let option = |id: &String| DefinitionOption { label: definition_label(id).into(), value: Some(id.clone()) };
        std::iter::once(DefinitionOption::all()).chain(self.known_definitions.iter().map(option)).collect()
    }
}

pub(super) fn definition_label(definition_id: &str) -> String {
    let mut parts = definition_id.split(':');
    let key = parts.next().unwrap_or_default();
    match parts.next() {
        Some(version) => tr!("incidents.definition_version", key = key, version = version).to_string(),
        None => key.to_string(),
    }
}

pub(super) fn skeleton() -> impl IntoElement {
    v_flex().gap_3().children((0..6).map(|_| Skeleton::new().h_10().rounded_md()))
}

pub(super) fn empty(nothing_loaded: bool) -> impl IntoElement {
    let header = if nothing_loaded {
        EmptyHeader::new().title(EmptyTitle::new().child(tr!("incidents.empty")))
    } else {
        EmptyHeader::new()
            .title(EmptyTitle::new().child(tr!("incidents.not_found")))
            .description(EmptyDescription::new().child(tr!("incidents.change_filter")))
    };
    Empty::new().size_full().header(header)
}
