mod catalog;
mod key;
mod process;
mod results;
mod table;
mod variables;

use camunda_core::models::{CamundaConnection, ProcessVariableCatalog};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::select::{SearchableVec, SelectEvent, SelectState};
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::table::{TableEvent, TableState};
use gpui_kit::component::{IndexPath, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model::Navigate;
use crate::platform::locale;
use catalog::{TypeItem, VariableItem, variable_items};
use process::ProcessItem;
use table::ResultTable;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Key,
    Variable,
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
    locale: SharedString,
    _definitions_load: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for SearchView {}

impl SearchView {
    pub fn new(connection: CamundaConnection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let key_query = cx.new(|cx| InputState::new(window, cx).placeholder(tr!("search.key.placeholder")));
        let value = cx.new(|cx| InputState::new(window, cx).placeholder(tr!("search.value.placeholder")));
        let process = cx.new(|cx| SelectState::new(SearchableVec::new(Vec::<ProcessItem>::new()), None, window, cx).searchable(true));
        let variable = cx.new(|cx| SelectState::new(SearchableVec::new(Vec::<VariableItem>::new()), None, window, cx).searchable(true));
        let value_type = cx.new(|cx| SelectState::new(TypeItem::all(), Some(IndexPath::default()), window, cx));
        let table = cx.new(|cx| TableState::new(ResultTable::default(), window, cx));
        let subscriptions = vec![
            cx.subscribe(&key_query, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.search_by_key(cx);
                }
            }),
            cx.subscribe(&value, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.search_by_variable(cx);
                }
            }),
            cx.subscribe_in(&process, window, |this, _, event: &SelectEvent<SearchableVec<ProcessItem>>, window, cx| {
                let SelectEvent::Confirm(key) = event;
                this.load_catalog(key.clone(), window, cx);
            }),
            cx.subscribe(&table, |_, table, event: &TableEvent, cx| {
                if let TableEvent::DoubleClickedRow(row) = event
                    && let Some(instance) = table.read(cx).delegate().rows.get(*row)
                {
                    cx.emit(Navigate::Instance(instance.id.clone()));
                }
            }),
        ];
        let mut view = Self {
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
            locale: locale::current(),
            _definitions_load: None,
            _subscriptions: subscriptions,
        };
        view._definitions_load = Some(view.load_definitions(window, cx));
        view
    }
}

impl SearchView {
    // После смены языка обновляет тексты, которые компоненты GPUI Kit держат у себя.
    fn relabel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = locale::current();
        if current == self.locale {
            return;
        }
        self.locale = current;
        self.key_query.update(cx, |state, cx| state.set_placeholder(tr!("search.key.placeholder"), window, cx));
        self.value.update(cx, |state, cx| state.set_placeholder(tr!("search.value.placeholder"), window, cx));
        self.value_type.update(cx, |state, cx| state.set_items(TypeItem::all(), window, cx));
        if let Some(catalog) = &self.catalog {
            let items = variable_items(catalog);
            self.variable.update(cx, |state, cx| state.set_items(SearchableVec::new(items), window, cx));
        }
        self.table.update(cx, |table, cx| table.refresh(cx));
    }
}

impl Render for SearchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.relabel(window, cx);
        let form = match self.mode {
            Mode::Key => self.render_key_form(cx).into_any_element(),
            Mode::Variable => self.render_variable_form(cx).into_any_element(),
        };
        let mode_tabs = TabBar::new("search-mode")
            .segmented()
            .selected_index(if self.mode == Mode::Key { 0 } else { 1 })
            .children([tr!("search.mode.key"), tr!("search.mode.variable")])
            .on_click(cx.listener(|this, index: &usize, _, cx| this.select_mode(if *index == 0 { Mode::Key } else { Mode::Variable }, cx)));
        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                h_flex()
                    .gap_4()
                    .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).child(tr!("search.title")))
                    .child(mode_tabs),
            )
            .child(form)
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("search-error", error)))
            .child(self.render_results(cx))
    }
}
