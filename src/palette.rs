use std::time::Duration;

use camunda_core::models::{CamundaConnection, ProcessInstanceSummary};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::services;
use crate::ui::{self, Tone};

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
    Dismiss,
}

const SECTIONS: [(Section, &str, &str); 4] = [
    (Section::Overview, "Обзор", "⌘1"),
    (Section::Search, "Поиск", "⌘2"),
    (Section::Incidents, "Инциденты", "⌘3"),
    (Section::Settings, "Настройки", "⌘,"),
];

pub struct CommandPalette {
    connection: Option<CamundaConnection>,
    query: Entity<InputState>,
    results: Vec<ProcessInstanceSummary>,
    selected: usize,
    searching: bool,
    error: Option<String>,
    search: Option<Task<()>>,
    _subscription: Subscription,
}

impl EventEmitter<PaletteEvent> for CommandPalette {}

impl CommandPalette {
    pub fn new(connection: Option<CamundaConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Бизнес-ключ или ID заявки"));
        query.update(cx, |state, cx| state.focus(window, cx));
        let subscription = cx.subscribe(&query, |this, _, event: &InputEvent, cx| match event {
            InputEvent::Change => this.schedule_search(cx),
            InputEvent::PressEnter { .. } => this.confirm(cx),
            _ => {}
        });
        Self {
            connection,
            query,
            results: Vec::new(),
            selected: 0,
            searching: false,
            error: None,
            search: None,
            _subscription: subscription,
        }
    }

    fn needle(&self, cx: &App) -> String {
        self.query.read(cx).value().trim().to_string()
    }

    fn schedule_search(&mut self, cx: &mut Context<Self>) {
        let needle = self.needle(cx);
        self.selected = 0;
        self.error = None;
        if needle.is_empty() || self.connection.is_none() {
            self.results.clear();
            self.searching = false;
            self.search = None;
            cx.notify();
            return;
        }
        let connection = self.connection.clone().expect("подключение выбрано");
        self.searching = true;
        self.search = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let Ok(request) = this.update(cx, |_, cx| {
                services::request(cx, move |api| async move { api.search_instances_by_business_key(&connection, &needle).await })
            }) else {
                return;
            };
            let result = request.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(results) => this.results = results,
                    Err(error) => this.error = Some(error),
                }
                this.searching = false;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn items_count(&self, cx: &App) -> usize {
        if self.needle(cx).is_empty() { SECTIONS.len() } else { self.results.len() }
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        if self.needle(cx).is_empty() {
            if let Some((section, _, _)) = SECTIONS.get(self.selected) {
                cx.emit(PaletteEvent::Go(*section));
            }
        } else if let Some(instance) = self.results.get(self.selected) {
            cx.emit(PaletteEvent::Open(instance.id.clone()));
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.items_count(cx);
        match event.keystroke.key.as_str() {
            "escape" => cx.emit(PaletteEvent::Dismiss),
            "down" if count > 0 => {
                self.selected = (self.selected + 1) % count;
                cx.notify();
            }
            "up" if count > 0 => {
                self.selected = (self.selected + count - 1) % count;
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    fn row(&self, id: SharedString, index: usize, cx: &App) -> Stateful<Div> {
        let selected = index == self.selected;
        h_flex()
            .id(id)
            .px_3()
            .py_2p5()
            .gap_3()
            .rounded_md()
            .cursor_pointer()
            .when(selected, |this| this.bg(cx.theme().table_active))
            .hover(|style| style.bg(cx.theme().table_hover))
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let empty = self.needle(cx).is_empty();
        let list: Vec<AnyElement> = if empty {
            SECTIONS
                .iter()
                .enumerate()
                .map(|(index, (section, label, shortcut))| {
                    let section = *section;
                    self.row(SharedString::from(format!("section-{label}")), index, cx)
                        .child(div().flex_1().font_weight(FontWeight::MEDIUM).child(*label))
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(*shortcut))
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(PaletteEvent::Go(section))))
                        .into_any_element()
                })
                .collect()
        } else {
            self.results
                .iter()
                .enumerate()
                .map(|(index, instance)| {
                    let id = instance.id.clone();
                    let process = instance.definition_name.clone().unwrap_or_else(|| instance.definition_key.clone());
                    self.row(SharedString::from(format!("result-{}", instance.id)), index, cx)
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap_0p5()
                                .child(
                                    div()
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(instance.business_key.clone().unwrap_or_else(|| "Без бизнес-ключа".into())),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_xs()
                                        .font_family(crate::theme::MONO)
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("{process} · {}", instance.id)),
                                ),
                        )
                        .child(ui::instance_state(instance.suspended, instance.ended, cx))
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(PaletteEvent::Open(id.clone()))))
                        .into_any_element()
                })
                .collect()
        };
        let footer: Option<SharedString> = if self.connection.is_none() {
            Some("Нет подключения: добавьте его в настройках".into())
        } else if self.searching {
            Some("Ищем…".into())
        } else if !empty && self.results.is_empty() && self.error.is_none() {
            Some("Ничего не найдено".into())
        } else {
            None
        };

        v_flex()
            .id("command-palette")
            .key_context("CommandPalette")
            .w(px(640.))
            .max_h(px(520.))
            .p_3()
            .gap_2()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .shadow_2xl()
            .occlude()
            .on_key_down(cx.listener(Self::on_key))
            .child(Input::new(&self.query).large())
            .child(div().px_1().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(cx.theme().muted_foreground).child(if empty {
                "Разделы"
            } else {
                "Заявки"
            }))
            .child(v_flex().id("palette-list").flex_1().min_h_0().overflow_y_scroll().gap_0p5().children(list))
            .when_some(self.error.clone(), |this, error| this.child(ui::pill(error, Tone::Danger, cx)))
            .when_some(footer, |this, footer| {
                this.child(div().px_1().py_1().text_sm().text_color(cx.theme().muted_foreground).child(footer))
            })
            .child(
                h_flex()
                    .px_1()
                    .gap_3()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("↑↓ выбрать")
                    .child("↵ открыть")
                    .child("esc закрыть"),
            )
    }
}
