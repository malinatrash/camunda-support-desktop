mod open;

use std::time::Duration;

use camunda_core::models::{CamundaConnection, ProcessInstanceSummary};
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::{ActiveTheme, IndexPath, h_flex, v_flex};
use gpui_kit::*;

use crate::model;
use crate::platform::services;

const DEBOUNCE: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    Overview,
    Search,
    Incidents,
    Settings,
}

pub enum PaletteEvent {
    Open(String),
    Go(Section),
}

const SECTIONS: [(Section, &str, &str); 4] = [
    (Section::Overview, "sidebar.overview", "cmd-1"),
    (Section::Search, "sidebar.search", "cmd-2"),
    (Section::Incidents, "sidebar.incidents", "cmd-3"),
    (Section::Settings, "sidebar.settings", "cmd-,"),
];

pub struct CommandPalette {
    connection: Option<CamundaConnection>,
    state: Entity<CommandState>,
    query: String,
    results: Vec<ProcessInstanceSummary>,
    error: Option<String>,
    search: Option<Task<()>>,
}

impl EventEmitter<PaletteEvent> for CommandPalette {}

impl CommandPalette {
    pub fn new(connection: Option<CamundaConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| CommandState::new(window, cx));
        let focus = state.read(cx).focus_handle(cx);
        focus.focus(window, cx);
        Self { connection, state, query: String::new(), results: Vec::new(), error: None, search: None }
    }

    fn on_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.query = query.trim().to_string();
        self.error = None;
        self.results.clear();
        let Some(connection) = self.connection.clone().filter(|_| !self.query.is_empty()) else {
            self.search = None;
            self.state.update(cx, |state, cx| state.set_loading(false, window, cx));
            cx.notify();
            return;
        };
        let needle = self.query.clone();
        self.state.update(cx, |state, cx| state.set_loading(true, window, cx));
        self.search = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let Ok(request) = this.update(cx, |_, cx| {
                services::request(cx, move |api| async move { api.search_instances_by_business_key(&connection, &needle).await })
            }) else {
                return;
            };
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(results) => this.results = results,
                    Err(error) => this.error = Some(error),
                }
                this.state.update(cx, |state, cx| state.set_loading(false, window, cx));
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn on_confirm(&mut self, index: IndexPath, cx: &mut Context<Self>) {
        if self.query.is_empty() {
            if let Some((section, _, _)) = SECTIONS.get(index.row) {
                cx.emit(PaletteEvent::Go(*section));
            }
        } else if let Some(instance) = self.results.get(index.row) {
            cx.emit(PaletteEvent::Open(instance.id.clone()));
        }
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let group = if self.query.is_empty() {
            CommandGroup::new().label(tr!("palette.sections")).items(SECTIONS.iter().map(|(_, key, keys)| {
                let (label, keys) = (tr!(*key), *keys);
                CommandItem::new().label(label.clone()).child(move |_, _| {
                    h_flex().w_full().child(div().flex_1().child(label.clone())).child(Kbd::new(Keystroke::parse(keys).expect("сочетание")))
                })
            }))
        } else {
            CommandGroup::new().label(tr!("palette.instances")).items(self.results.iter().map(|instance| {
                let instance = instance.clone();
                CommandItem::new().label(instance.business_key.clone().unwrap_or_default()).child(move |_, cx| {
                    let process = instance.definition_name.clone().unwrap_or_else(|| instance.definition_key.clone());
                    h_flex()
                        .w_full()
                        .gap_3()
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .child(div().truncate().child(
                                    instance.business_key.clone().map(SharedString::from).unwrap_or_else(|| tr!("palette.no_business_key")),
                                ))
                                .child(
                                    div()
                                        .truncate()
                                        .text_xs()
                                        .font_family(crate::platform::theme::mono(cx))
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{process} · {}", instance.id)),
                                ),
                        )
                        .child(model::state_tag(instance.suspended, instance.ended))
                })
            }))
        };
        let empty: SharedString = if self.connection.is_none() {
            tr!("palette.no_connection")
        } else if let Some(error) = &self.error {
            error.clone().into()
        } else {
            tr!("palette.nothing_found")
        };
        let query_view = cx.entity().downgrade();
        let confirm_view = query_view.clone();
        Command::new(&self.state)
            .placeholder(tr!("palette.placeholder"))
            .filterable(false)
            .max_h(px(420.))
            .group(group)
            .empty(move |_, _, _| div().p_4().child(empty.clone()))
            .on_query(move |query, window, cx| {
                query_view.update(cx, |this, cx| this.on_query(query, window, cx)).ok();
            })
            .on_confirm(move |index, _, cx| {
                confirm_view.update(cx, |this, cx| this.on_confirm(index, cx)).ok();
            })
    }
}
