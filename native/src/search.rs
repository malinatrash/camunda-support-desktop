use camunda_core::models::{
    CamundaConnection, ProcessDefinitionSummary, ProcessInstanceSummary, ProcessVariableCatalog, VariableValueType,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme, Disableable, IconName, IndexPath, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model;
use crate::services;
use crate::ui::{self, Navigate};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Key,
    Variable,
}

#[derive(Clone)]
pub struct ProcessItem {
    key: SharedString,
    title: SharedString,
    subtitle: SharedString,
}

impl SelectItem for ProcessItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .child(self.title.clone())
            .child(div().text_xs().font_family(crate::theme::MONO).text_color(cx.theme().muted_foreground).child(self.subtitle.clone()))
    }

    fn value(&self) -> &SharedString {
        &self.key
    }

    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.title.to_lowercase().contains(&query) || self.key.to_lowercase().contains(&query)
    }
}

#[derive(Clone)]
pub struct VariableItem {
    name: SharedString,
    subtitle: SharedString,
}

impl SelectItem for VariableItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .child(div().font_family(crate::theme::MONO).child(self.name.clone()))
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(self.subtitle.clone()))
    }

    fn value(&self) -> &SharedString {
        &self.name
    }
}

#[derive(Clone)]
pub struct TypeItem {
    label: SharedString,
    value: VariableValueType,
}

impl SelectItem for TypeItem {
    type Value = VariableValueType;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &VariableValueType {
        &self.value
    }
}

fn type_items() -> Vec<TypeItem> {
    [
        ("Авто", VariableValueType::Auto),
        ("Строка", VariableValueType::String),
        ("Число", VariableValueType::Number),
        ("Булево", VariableValueType::Boolean),
    ]
    .into_iter()
    .map(|(label, value)| TypeItem { label: label.into(), value })
    .collect()
}

fn process_item(definition: &ProcessDefinitionSummary) -> ProcessItem {
    ProcessItem {
        key: definition.key.clone().into(),
        title: model::display_name(definition).to_string().into(),
        subtitle: format!("{} · v{}", definition.key, definition.version).into(),
    }
}

pub struct SearchView {
    connection: CamundaConnection,
    mode: Mode,
    key_query: Entity<InputState>,
    process: Entity<SelectState<SearchableVec<ProcessItem>>>,
    variable: Entity<SelectState<SearchableVec<VariableItem>>>,
    value: Entity<InputState>,
    value_type: Entity<SelectState<Vec<TypeItem>>>,
    catalog: Option<ProcessVariableCatalog>,
    definitions_error: Option<String>,
    catalog_error: Option<String>,
    error: Option<String>,
    results: Option<usize>,
    table: Entity<TableState<ResultTable>>,
    search: Option<Task<()>>,
    catalog_load: Option<Task<()>>,
    _definitions_load: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for SearchView {}

impl SearchView {
    pub fn new(connection: CamundaConnection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let key_query = cx.new(|cx| InputState::new(window, cx).placeholder("Бизнес-ключ целиком или его часть, либо ID заявки"));
        let value = cx.new(|cx| InputState::new(window, cx).placeholder("Точное значение"));
        let process = cx.new(|cx| SelectState::new(SearchableVec::new(Vec::<ProcessItem>::new()), None, window, cx).searchable(true));
        let variable = cx.new(|cx| SelectState::new(SearchableVec::new(Vec::<VariableItem>::new()), None, window, cx).searchable(true));
        let value_type = cx.new(|cx| SelectState::new(type_items(), Some(IndexPath::default()), window, cx));
        let table = cx.new(|cx| TableState::new(ResultTable::default(), window, cx));

        let subscriptions = vec![
            cx.subscribe_in(&key_query, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.search_by_key(cx);
                }
            }),
            cx.subscribe_in(&value, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.search_by_variable(cx);
                }
            }),
            cx.subscribe_in(&process, window, |this, _, event: &SelectEvent<SearchableVec<ProcessItem>>, window, cx| {
                let SelectEvent::Confirm(key) = event;
                this.load_catalog(key.clone(), window, cx);
            }),
            cx.subscribe(&table, |_, table, event: &TableEvent, cx| {
                if let TableEvent::DoubleClickedRow(row) = event {
                    if let Some(instance) = table.read(cx).delegate().rows.get(*row) {
                        cx.emit(Navigate::Instance(instance.id.clone()));
                    }
                }
            }),
        ];

        let definitions_connection = connection.clone();
        let request = services::request(cx, move |api| async move { api.load_search_process_definitions(&definitions_connection).await });
        let definitions_load = cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(definitions) => {
                        let items: Vec<ProcessItem> = definitions.iter().map(process_item).collect();
                        this.process.update(cx, |state, cx| state.set_items(SearchableVec::new(items), window, cx));
                    }
                    Err(error) => this.definitions_error = Some(error),
                }
                cx.notify();
            })
            .ok();
        });

        Self {
            connection,
            mode: Mode::Key,
            key_query,
            process,
            variable,
            value,
            value_type,
            catalog: None,
            definitions_error: None,
            catalog_error: None,
            error: None,
            results: None,
            table,
            search: None,
            catalog_load: None,
            _definitions_load: definitions_load,
            _subscriptions: subscriptions,
        }
    }

    fn selected_process(&self, cx: &App) -> Option<String> {
        self.process.read(cx).selected_value().map(|key| key.to_string())
    }

    fn load_catalog(&mut self, key: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        self.catalog = None;
        self.catalog_error = None;
        self.variable.update(cx, |state, cx| state.set_items(SearchableVec::new(Vec::<VariableItem>::new()), window, cx));
        let Some(key) = key else {
            self.catalog_load = None;
            cx.notify();
            return;
        };
        let connection = self.connection.clone();
        let request = services::request(cx, move |api| async move { api.load_process_variable_catalog(&connection, &key).await });
        self.catalog_load = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(catalog) => {
                        let items: Vec<VariableItem> = catalog
                            .variables
                            .iter()
                            .map(|variable| VariableItem {
                                name: variable.name.clone().into(),
                                subtitle: format!("{} · встречается {} раз", variable.types.join(", "), variable.occurrences).into(),
                            })
                            .collect();
                        this.variable.update(cx, |state, cx| state.set_items(SearchableVec::new(items), window, cx));
                        this.catalog = Some(catalog);
                    }
                    Err(error) => this.catalog_error = Some(error),
                }
                this.catalog_load = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    // Сбрасывает кэш переменных процесса и читает их заново.
    fn refresh_catalog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.selected_process(cx) else { return };
        cx.global::<services::Services>().api.invalidate_process_variable_catalog(&self.connection, &key);
        self.load_catalog(Some(key.into()), window, cx);
    }

    fn show_results(&mut self, result: Result<Vec<ProcessInstanceSummary>, String>, cx: &mut Context<Self>) {
        match result {
            Ok(rows) => {
                self.results = Some(rows.len());
                self.table.update(cx, |table, cx| {
                    table.delegate_mut().rows = rows;
                    table.refresh(cx);
                });
            }
            Err(error) => self.error = Some(error),
        }
        self.search = None;
        cx.notify();
    }

    fn search_by_key(&mut self, cx: &mut Context<Self>) {
        let query = self.key_query.read(cx).value().trim().to_string();
        if query.is_empty() {
            self.error = Some("Введите бизнес-ключ или ID заявки.".into());
            cx.notify();
            return;
        }
        let connection = self.connection.clone();
        let request = services::request(cx, move |api| async move { api.search_instances_by_business_key(&connection, &query).await });
        self.start(request, cx);
    }

    fn search_by_variable(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.selected_process(cx) else {
            self.error = Some("Сначала выберите процесс.".into());
            cx.notify();
            return;
        };
        let Some(name) = self.variable.read(cx).selected_value().map(|name| name.to_string()) else {
            self.error = Some("Выберите переменную из списка процесса.".into());
            cx.notify();
            return;
        };
        let value = self.value.read(cx).value().to_string();
        if value.trim().is_empty() {
            self.error = Some("Укажите точное значение переменной.".into());
            cx.notify();
            return;
        }
        let value_type = self.value_type.read(cx).selected_value().copied().unwrap_or(VariableValueType::Auto);
        let connection = self.connection.clone();
        let request = services::request(cx, move |api| async move {
            api.search_process_instances(&connection, &key, &name, &value, value_type).await
        });
        self.start(request, cx);
    }

    fn start(&mut self, request: impl Future<Output = Result<Vec<ProcessInstanceSummary>, String>> + 'static, cx: &mut Context<Self>) {
        self.error = None;
        self.search = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| this.show_results(result, cx)).ok();
        }));
        cx.notify();
    }

    fn select_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.error = None;
        self.results = None;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().rows.clear();
            table.refresh(cx);
        });
        cx.notify();
    }

    fn render_key_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.key_query)))
                    .child(
                        Button::new("search-key")
                            .label("Найти")
                            .primary()
                            .loading(self.search.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.search_by_key(cx))),
                    ),
            )
            .child(hint("Подстрока бизнес-ключа или ID заявки — ищем среди запущенных и завершённых.", cx))
    }

    fn render_variable_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let process_selected = self.selected_process(cx).is_some();
        let step: String = if !process_selected {
            "1. Выберите процесс".into()
        } else if self.catalog_load.is_some() {
            "2. Читаем переменные активных заявок…".into()
        } else if let Some(catalog) = &self.catalog {
            format!(
                "2. Найдено ключей: {} в {} активных заявках{} · кэш между сессиями до 24 ч.",
                catalog.variables.len(),
                catalog.inspected_instance_count,
                if catalog.instances_truncated { " (выборка ограничена)" } else { "" }
            )
        } else {
            "2. Выберите переменную".into()
        };
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div().w(px(300.)).flex_none().child(
                            Select::new(&self.process)
                                .placeholder("Процесс")
                                .search_placeholder("Название или ключ процесса")
                                .menu_width(px(420.))
                                .accessibility_label("Процесс"),
                        ),
                    )
                    .child(
                        div().w(px(260.)).flex_none().child(
                            Select::new(&self.variable)
                                .placeholder("Переменная")
                                .search_placeholder("Имя переменной")
                                .menu_width(px(380.))
                                .accessibility_label("Переменная")
                                .disabled(!process_selected || self.catalog_load.is_some()),
                        ),
                    )
                    .child(
                        Button::new("refresh-catalog")
                            .icon(IconName::RefreshCw)
                            .ghost()
                            .tooltip("Перечитать переменные процесса")
                            .loading(self.catalog_load.is_some())
                            .disabled(!process_selected)
                            .on_click(cx.listener(|this, _, window, cx| this.refresh_catalog(window, cx))),
                    )
                    .child(div().flex_1().min_w(px(160.)).child(Input::new(&self.value)))
                    .child(div().w(px(120.)).flex_none().child(Select::new(&self.value_type).accessibility_label("Тип значения")))
                    .child(
                        Button::new("search-variable")
                            .label("Найти")
                            .primary()
                            .loading(self.search.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.search_by_variable(cx))),
                    ),
            )
            .child(hint(step, cx))
            .when_some(self.definitions_error.clone().or_else(|| self.catalog_error.clone()), |this, error| this.child(error_box(error, cx)))
    }
}

fn hint(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div().text_sm().text_color(cx.theme().muted_foreground).child(text.into())
}

fn error_box(error: String, cx: &App) -> impl IntoElement {
    div().px_4().py_2().rounded_md().bg(cx.theme().danger.opacity(0.1)).text_color(cx.theme().danger).child(error)
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let form = match self.mode {
            Mode::Key => self.render_key_form(cx).into_any_element(),
            Mode::Variable => self.render_variable_form(cx).into_any_element(),
        };
        let results = match self.results {
            Some(0) => div()
                .p_6()
                .text_color(cx.theme().muted_foreground)
                .child(match self.mode {
                    Mode::Key => "Заявки не найдены. Проверьте бизнес-ключ или ID.",
                    Mode::Variable => "Активных заявок с таким значением не найдено.",
                })
                .into_any_element(),
            Some(count) => ui::section(
                h_flex()
                    .w_full()
                    .gap_2()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Найденные заявки"))
                    .child(ui::counter(count, cx))
                    .child(div().flex_1())
                    .child(hint("Двойной клик открывает заявку", cx)),
                cx,
            )
            .flex_1()
            .child(div().flex_1().min_h_0().child(DataTable::new(&self.table).large().stripe(false).bordered(false)))
            .into_any_element(),
            None => div().into_any_element(),
        };
        let mode_index = if self.mode == Mode::Key { 0 } else { 1 };

        v_flex()
            .size_full()
            .p_5()
            .gap_4()
            .child(
                TabBar::new("search-mode")
                    .segmented()
                    .selected_index(mode_index)
                    .children(["Бизнес-ключ или ID", "По переменной"])
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.select_mode(if *index == 0 { Mode::Key } else { Mode::Variable }, cx);
                    })),
            )
            .child(form)
            .when_some(self.error.clone(), |this, error| this.child(error_box(error, cx)))
            .child(results)
    }
}

pub struct ResultTable {
    rows: Vec<ProcessInstanceSummary>,
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
        match self.columns[col_ix].key.as_ref() {
            "state" => ui::instance_state(instance.suspended, instance.ended, cx).into_any_element(),
            "business_key" => div()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .child(instance.business_key.clone().unwrap_or_else(|| "—".into()))
                .into_any_element(),
            "process" => v_flex()
                .min_w_0()
                .child(div().truncate().child(instance.definition_name.clone().unwrap_or_else(|| instance.definition_key.clone())))
                .child(
                    div()
                        .truncate()
                        .text_xs()
                        .font_family(crate::theme::MONO)
                        .text_color(cx.theme().muted_foreground)
                        .child(instance.definition_id.clone()),
                )
                .into_any_element(),
            "id" => div().truncate().text_sm().font_family(crate::theme::MONO).child(instance.id.clone()).into_any_element(),
            _ => div()
                .text_sm()
                .font_family(crate::theme::MONO)
                .text_color(cx.theme().muted_foreground)
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
