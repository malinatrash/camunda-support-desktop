use std::collections::{BTreeMap, HashSet};

use camunda_core::models::{CamundaConnection, DashboardDateFilter, DashboardDatePreset, Environment, ProcessIncident};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model;
use crate::services;
use crate::ui::{self, Navigate, Tone};

const RETRIES: i32 = 3;

#[derive(Clone, Copy, PartialEq)]
enum View {
    Groups,
    List,
}

#[derive(Clone)]
pub struct DefinitionOption {
    label: SharedString,
    value: Option<String>,
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

// Группа инцидентов с самими инцидентами для повтора.
struct Group {
    summary: model::IncidentGroup,
    incidents: Vec<ProcessIncident>,
}

fn group_key(incident: &ProcessIncident) -> String {
    let activity = incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_default();
    let process_key = incident.process_definition_id.as_deref().and_then(|id| id.split(':').next()).unwrap_or_default();
    format!("{process_key}|{activity}|{}|{}", incident.incident_type, model::incident_signature(incident.message.as_deref()))
}

fn retryable(incident: &ProcessIncident) -> bool {
    matches!(incident.incident_type.as_str(), "failedJob" | "failedExternalTask") && incident.configuration.is_some()
}

fn definition_label(definition_id: &str) -> String {
    let mut parts = definition_id.split(':');
    let key = parts.next().unwrap_or_default();
    match parts.next() {
        Some(version) => format!("{key} · версия {version}"),
        None => key.to_string(),
    }
}

pub struct IncidentsView {
    connection: CamundaConnection,
    preset: DashboardDatePreset,
    incidents: Vec<ProcessIncident>,
    known_definitions: Vec<String>,
    definition: Entity<SelectState<Vec<DefinitionOption>>>,
    query: Entity<InputState>,
    selected_type: Option<String>,
    view: View,
    expanded_group: Option<SharedString>,
    table: Entity<TableState<IncidentTable>>,
    pending_retry: Option<Vec<ProcessIncident>>,
    confirmed: bool,
    notice: Option<SharedString>,
    error: Option<String>,
    load: Option<Task<()>>,
    retry: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for IncidentsView {}

impl IncidentsView {
    pub fn new(connection: CamundaConnection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Сообщение, заявка, кубик или ID"));
        let all = DefinitionOption { label: "Все процессы".into(), value: None };
        let definition = cx.new(|cx| SelectState::new(vec![all], Some(IndexPath::default()), window, cx));
        let table = cx.new(|cx| TableState::new(IncidentTable::default(), window, cx));
        let subscriptions = vec![
            cx.subscribe(&query, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.sync_table(cx);
                }
            }),
            cx.subscribe_in(&definition, window, |this, _, event: &SelectEvent<Vec<DefinitionOption>>, window, cx| {
                let SelectEvent::Confirm(_) = event;
                this.reload(window, cx);
            }),
            cx.subscribe(&table, |_, table, event: &TableEvent, cx| {
                if let TableEvent::DoubleClickedRow(row) = event
                    && let Some(id) = table.read(cx).delegate().rows.get(*row).and_then(|incident| incident.process_instance_id.clone())
                {
                    cx.emit(Navigate::Instance(id));
                }
            }),
        ];
        let mut view = Self {
            connection,
            preset: DashboardDatePreset::Live,
            incidents: Vec::new(),
            known_definitions: Vec::new(),
            definition,
            query,
            selected_type: None,
            view: View::Groups,
            expanded_group: None,
            table,
            pending_retry: None,
            confirmed: false,
            notice: None,
            error: None,
            load: None,
            retry: None,
            _subscriptions: subscriptions,
        };
        view.reload(window, cx);
        view
    }

    fn production(&self) -> bool {
        self.connection.environment == Environment::Production
    }

    fn selected_definition(&self, cx: &App) -> Option<String> {
        self.definition.read(cx).selected_value().cloned().flatten()
    }

    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    // Пополняет список версий процессов из загруженных инцидентов, не сбрасывая выбор.
    fn sync_definitions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;
        for id in self.incidents.iter().filter_map(|incident| incident.process_definition_id.clone()) {
            if !self.known_definitions.contains(&id) {
                self.known_definitions.push(id);
                changed = true;
            }
        }
        if !changed {
            return;
        }
        self.known_definitions.sort();
        let selected = self.selected_definition(cx);
        let mut options = vec![DefinitionOption { label: "Все процессы".into(), value: None }];
        options.extend(
            self.known_definitions.iter().map(|id| DefinitionOption { label: definition_label(id).into(), value: Some(id.clone()) }),
        );
        let index = selected
            .as_ref()
            .and_then(|id| self.known_definitions.iter().position(|known| known == id))
            .map(|position| position + 1)
            .unwrap_or(0);
        self.definition.update(cx, |state, cx| {
            state.set_items(options, window, cx);
            state.set_selected_index(Some(IndexPath::default().row(index)), window, cx);
        });
    }

    fn filtered(&self, cx: &App) -> Vec<ProcessIncident> {
        let query = self.query.read(cx).value().trim().to_lowercase();
        self.incidents
            .iter()
            .filter(|incident| self.selected_type.as_ref().is_none_or(|kind| &incident.incident_type == kind))
            .filter(|incident| {
                query.is_empty()
                    || [
                        Some(&incident.id),
                        incident.message.as_ref(),
                        incident.activity_id.as_ref(),
                        incident.failed_activity_id.as_ref(),
                        incident.process_instance_id.as_ref(),
                        incident.process_definition_id.as_ref(),
                        incident.configuration.as_ref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|value| value.to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }

    fn sync_table(&mut self, cx: &mut Context<Self>) {
        let rows = self.filtered(cx);
        self.table.update(cx, |table, cx| {
            let delegate = table.delegate_mut();
            let visible: HashSet<&str> = rows.iter().map(|incident| incident.id.as_str()).collect();
            delegate.selected.retain(|id| visible.contains(id.as_str()));
            delegate.rows = rows;
            table.refresh(cx);
        });
        cx.notify();
    }

    fn groups(&self, filtered: &[ProcessIncident]) -> Vec<Group> {
        let mut by_key: BTreeMap<String, Vec<ProcessIncident>> = BTreeMap::new();
        for incident in filtered {
            by_key.entry(group_key(incident)).or_default().push(incident.clone());
        }
        model::group_incidents(filtered)
            .into_iter()
            .map(|summary| Group { incidents: by_key.remove(summary.key.as_ref()).unwrap_or_default(), summary })
            .collect()
    }

    fn select_preset(&mut self, preset: DashboardDatePreset, window: &mut Window, cx: &mut Context<Self>) {
        self.preset = preset;
        self.reload(window, cx);
    }

    fn ask_retry(&mut self, incidents: Vec<ProcessIncident>, cx: &mut Context<Self>) {
        self.pending_retry = Some(incidents);
        self.confirmed = false;
        cx.notify();
    }

    fn confirm_retry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(incidents) = self.pending_retry.clone() else { return };
        let connection = self.connection.clone();
        let request = services::request(cx, move |api| async move { api.retry_incidents(&connection, &incidents, RETRIES).await });
        self.retry = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                this.retry = None;
                match result {
                    Ok(result) => {
                        let skipped = if result.skipped > 0 { format!(", пропущено {}", result.skipped) } else { String::new() };
                        this.notice = Some(
                            format!("Повтор запущен: заданий {}, внешних задач {}{skipped}.", result.jobs, result.external_tasks).into(),
                        );
                        this.pending_retry = None;
                        this.table.update(cx, |table, _| table.delegate_mut().selected.clear());
                        this.reload(window, cx);
                    }
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let presets =
            [(DashboardDatePreset::Live, "Сейчас"), (DashboardDatePreset::Today, "Сегодня"), (DashboardDatePreset::Yesterday, "Вчера")];
        h_flex()
            .gap_2()
            .children(presets.into_iter().map(|(preset, label)| {
                Button::new(label)
                    .label(label)
                    .small()
                    .map(|button| if self.preset == preset { button.secondary() } else { button.ghost() })
                    .on_click(cx.listener(move |this, _, window, cx| this.select_preset(preset, window, cx)))
            }))
            .child(
                div()
                    .w(px(260.))
                    .flex_none()
                    .child(Select::new(&self.definition).small().menu_width(px(320.)).accessibility_label("Процесс")),
            )
            .child(div().flex_1())
            .child(div().w(px(300.)).child(Input::new(&self.query).small()))
            .child(
                Button::new("reload")
                    .icon(IconName::RefreshCw)
                    .ghost()
                    .small()
                    .loading(self.load.is_some())
                    .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
            )
    }

    fn render_bar(&self, filtered: usize, groups: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let mut types: Vec<(String, usize)> = Vec::new();
        for incident in &self.incidents {
            match types.iter_mut().find(|(kind, _)| kind == &incident.incident_type) {
                Some((_, count)) => *count += 1,
                None => types.push((incident.incident_type.clone(), 1)),
            }
        }
        types.sort_by_key(|item| std::cmp::Reverse(item.1));
        let selected_count = self.table.read(cx).delegate().selected.len();
        let retryable_selected = self.selected_retryable(cx).len();
        h_flex()
            .gap_2()
            .pb_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                TabBar::new("incident-view")
                    .segmented()
                    .small()
                    .selected_index(if self.view == View::Groups { 0 } else { 1 })
                    .child(format!("Группы {groups}"))
                    .child(format!("Список {filtered}"))
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.view = if *index == 0 { View::Groups } else { View::List };
                        cx.notify();
                    })),
            )
            .child(div().w(px(1.)).h(px(16.)).mx_1().bg(cx.theme().border))
            .child(self.type_chip(None, "Все типы".into(), None, cx))
            .children(types.into_iter().take(6).map(|(kind, count)| {
                let label: SharedString = model::incident_type_label(&kind).to_string().into();
                self.type_chip(Some(kind), label, Some(count), cx)
            }))
            .child(div().flex_1())
            .when(self.view == View::List, |this| {
                this.child(
                    Button::new("retry-selected")
                        .label(format!("Повторить выбранные ({retryable_selected})"))
                        .primary()
                        .small()
                        .disabled(retryable_selected == 0 || selected_count == 0)
                        .on_click(cx.listener(|this, _, _, cx| {
                            let incidents = this.selected_retryable(cx);
                            this.ask_retry(incidents, cx);
                        })),
                )
            })
    }

    fn selected_retryable(&self, cx: &App) -> Vec<ProcessIncident> {
        let delegate = self.table.read(cx).delegate();
        delegate.rows.iter().filter(|incident| delegate.selected.contains(&incident.id) && retryable(incident)).cloned().collect()
    }

    fn type_chip(&self, kind: Option<String>, label: SharedString, count: Option<usize>, cx: &mut Context<Self>) -> AnyElement {
        let active = self.selected_type == kind;
        h_flex()
            .id(SharedString::from(format!("type-{}", kind.as_deref().unwrap_or("all"))))
            .h(px(28.))
            .px_3()
            .gap_1p5()
            .rounded_full()
            .text_sm()
            .cursor_pointer()
            .text_color(if active { cx.theme().foreground } else { cx.theme().muted_foreground })
            .when(active, |this| this.bg(cx.theme().secondary).font_weight(FontWeight::SEMIBOLD))
            .hover(|style| style.bg(cx.theme().table_hover))
            .child(label)
            .children(count.map(|count| ui::counter(count, cx)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_type = kind.clone();
                this.sync_table(cx);
            }))
            .into_any_element()
    }

    fn render_groups(&self, groups: Vec<Group>, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("incident-groups")
            .flex_1()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .overflow_y_scroll()
            .children(groups.into_iter().map(|group| self.render_group(group, cx)))
    }

    fn render_group(&self, group: Group, cx: &mut Context<Self>) -> AnyElement {
        let summary = &group.summary;
        let open = self.expanded_group.as_ref() == Some(&summary.key);
        let key = summary.key.clone();
        let retry: Vec<ProcessIncident> = group.incidents.iter().filter(|incident| retryable(incident)).cloned().collect();
        let retry_count = retry.len();
        let meta = format!(
            "{} · {} · заявок {} · {}",
            summary.process_key,
            model::incident_type_label(&summary.incident_type),
            summary.instance_ids.len(),
            model::short_time(summary.latest.as_deref())
        );
        let mut incidents = group.incidents;
        incidents.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let total = incidents.len();
        v_flex()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(open, |this| this.bg(cx.theme().table_head))
            .child(
                h_flex()
                    .id(summary.key.clone())
                    .px_4()
                    .py_3()
                    .gap_3()
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.theme().table_hover))
                    .child(
                        Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight })
                            .small()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(div().w(px(44.)).flex_none().flex().justify_end().child(ui::pill(summary.count.to_string(), Tone::Danger, cx)))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_0p5()
                            .child(div().truncate().font_weight(FontWeight::MEDIUM).child(summary.activity.clone()))
                            .child(div().truncate().text_sm().child(summary.signature.clone()))
                            .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(meta)),
                    )
                    .child(
                        Button::new(SharedString::from(format!("retry-{}", summary.key)))
                            .label(format!("Повторить ({retry_count})"))
                            .ghost()
                            .small()
                            .disabled(retry_count == 0)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.ask_retry(retry.clone(), cx);
                            })),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.expanded_group = if this.expanded_group.as_ref() == Some(&key) { None } else { Some(key.clone()) };
                        cx.notify();
                    })),
            )
            .when(open, |this| {
                this.child(
                    v_flex()
                        .pl(px(104.))
                        .pr_4()
                        .pb_3()
                        .gap_1()
                        .children(incidents.iter().take(50).map(|incident| {
                            let target = incident.process_instance_id.clone();
                            h_flex()
                                .gap_4()
                                .text_sm()
                                .child(
                                    div()
                                        .w(px(150.))
                                        .flex_none()
                                        .font_family(crate::theme::MONO)
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(model::short_time(incident.timestamp.as_deref())),
                                )
                                .child(
                                    div()
                                        .id(SharedString::from(format!("open-{}", incident.id)))
                                        .font_family(crate::theme::MONO)
                                        .text_color(cx.theme().link)
                                        .cursor_pointer()
                                        .hover(|style| style.underline())
                                        .child(incident.process_instance_id.clone().unwrap_or_else(|| "—".into()))
                                        .on_click(cx.listener(move |_, _, _, cx| {
                                            if let Some(id) = &target {
                                                cx.emit(Navigate::Instance(id.clone()));
                                            }
                                        })),
                                )
                        }))
                        .when(total > 50, |this| {
                            this.child(
                                div().text_xs().text_color(cx.theme().muted_foreground).child(format!("Показаны первые 50 из {total}")),
                            )
                        }),
                )
            })
            .into_any_element()
    }

    fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let delegate = self.table.read(cx).delegate();
        let all = !delegate.rows.is_empty() && delegate.selected.len() == delegate.rows.len();
        ui::section(
            Checkbox::new("pick-all").label("Выбрать все").checked(all).on_click(cx.listener(move |this, checked: &bool, _, cx| {
                let checked = *checked;
                this.table.update(cx, |table, cx| {
                    let delegate = table.delegate_mut();
                    delegate.selected =
                        if checked { delegate.rows.iter().map(|incident| incident.id.clone()).collect() } else { HashSet::new() };
                    table.refresh(cx);
                });
                cx.notify();
            })),
            cx,
        )
        .flex_1()
        .child(div().flex_1().min_h_0().child(DataTable::new(&self.table).large().stripe(false).bordered(false)))
    }

    fn render_confirm(&self, incidents: &[ProcessIncident], cx: &mut Context<Self>) -> impl IntoElement {
        let production = self.production();
        let can_run = self.retry.is_none() && (!production || self.confirmed);
        div().absolute().inset_0().flex().items_center().justify_center().bg(black().opacity(0.45)).child(
            v_flex()
                .id("retry-confirm")
                .w(px(520.))
                .p_5()
                .gap_4()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().popover)
                .shadow_2xl()
                .occlude()
                .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Повторить инциденты"))
                .child(div().text_sm().child(format!(
                    "Выставить {RETRIES} попытки для {} инцидентов? Camunda сразу начнёт повторное выполнение заданий и внешних задач.",
                    incidents.len()
                )))
                .when(production, |this| {
                    this.child(Checkbox::new("confirm-production").label("Понимаю, что это прод").checked(self.confirmed).on_click(
                        cx.listener(|this, checked: &bool, _, cx| {
                            this.confirmed = *checked;
                            cx.notify();
                        }),
                    ))
                })
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("retry-cancel").label("Отмена").ghost().disabled(self.retry.is_some()).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.pending_retry = None;
                                cx.notify();
                            },
                        )))
                        .child(
                            Button::new("retry-run")
                                .label("Повторить")
                                .map(|button| if production { button.danger() } else { button.primary() })
                                .loading(self.retry.is_some())
                                .disabled(!can_run)
                                .on_click(cx.listener(|this, _, window, cx| this.confirm_retry(window, cx))),
                        ),
                ),
        )
    }
}

impl Render for IncidentsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filtered = self.filtered(cx);
        let groups = self.groups(&filtered);
        let empty = if self.incidents.is_empty() {
            "Инцидентов нет"
        } else {
            "Ничего не найдено — измените фильтр"
        };
        let body = if self.load.is_some() && self.incidents.is_empty() {
            div().p_6().text_color(cx.theme().muted_foreground).child("Загружаем инциденты…").into_any_element()
        } else if filtered.is_empty() {
            div().p_6().text_color(cx.theme().muted_foreground).child(empty).into_any_element()
        } else if self.view == View::Groups {
            self.render_groups(groups, cx).into_any_element()
        } else {
            self.render_list(cx).into_any_element()
        };
        let group_count = model::group_incidents(&filtered).len();

        v_flex()
            .relative()
            .size_full()
            .p_5()
            .gap_4()
            .child(self.render_toolbar(cx))
            .child(self.render_bar(filtered.len(), group_count, cx))
            .when_some(self.notice.clone(), |this, notice| {
                this.child(
                    div().px_4().py_2().rounded_md().bg(cx.theme().success.opacity(0.12)).text_color(cx.theme().success).child(notice),
                )
            })
            .when_some(self.error.clone(), |this, error| {
                this.child(div().px_4().py_2().rounded_md().bg(cx.theme().danger.opacity(0.1)).text_color(cx.theme().danger).child(error))
            })
            .child(body)
            .when_some(self.pending_retry.clone(), |this, incidents| this.child(deferred(self.render_confirm(&incidents, cx))))
    }
}

pub struct IncidentTable {
    rows: Vec<ProcessIncident>,
    selected: HashSet<String>,
    columns: Vec<Column>,
}

impl Default for IncidentTable {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            selected: HashSet::new(),
            columns: vec![
                Column::new("pick", "").width(44.).resizable(false),
                Column::new("message", "Сообщение").width(520.),
                Column::new("type", "Тип").width(180.),
                Column::new("activity", "Кубик").width(200.),
                Column::new("process", "Процесс").width(180.),
                Column::new("time", "Время").width(160.),
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
            "message" => div()
                .truncate()
                .child(incident.message.clone().unwrap_or_else(|| "Camunda не вернула сообщение ошибки".into()))
                .into_any_element(),
            "type" => div()
                .truncate()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(model::incident_type_label(&incident.incident_type).to_string())
                .into_any_element(),
            "activity" => div()
                .truncate()
                .text_sm()
                .font_family(crate::theme::MONO)
                .child(incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_else(|| "—".into()))
                .into_any_element(),
            "process" => div()
                .truncate()
                .text_sm()
                .child(incident.process_definition_id.as_deref().map(definition_label).unwrap_or_else(|| "—".into()))
                .into_any_element(),
            _ => div()
                .text_sm()
                .font_family(crate::theme::MONO)
                .text_color(cx.theme().muted_foreground)
                .child(model::short_time(incident.timestamp.as_deref()))
                .into_any_element(),
        }
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<SharedString> {
        let incident = self.rows.get(row_ix)?;
        match self.columns[col_ix].key.as_ref() {
            "message" => incident.message.clone().map(Into::into),
            "activity" => incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).map(Into::into),
            _ => None,
        }
    }
}
