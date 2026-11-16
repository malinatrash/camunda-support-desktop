mod details;
mod filters;
mod groups;
mod list;
mod load;
mod retry;
mod table;

use camunda_core::models::{CamundaConnection, DashboardDatePreset, ProcessIncident};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::select::{SelectEvent, SelectState};
use gpui_kit::component::table::{TableEvent, TableState};
use gpui_kit::component::{IndexPath, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model::Navigate;
use crate::platform::locale;
use load::DefinitionOption;
use table::IncidentTable;

pub struct IncidentsView {
    connection: CamundaConnection,
    preset: DashboardDatePreset,
    incidents: Vec<ProcessIncident>,
    known_definitions: Vec<String>,
    definition: Entity<SelectState<Vec<DefinitionOption>>>,
    query: Entity<InputState>,
    selected_type: Option<String>,
    selected_group: Option<SharedString>,
    table: Entity<TableState<IncidentTable>>,
    notice: Option<SharedString>,
    error: Option<String>,
    load: Option<Task<()>>,
    locale: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for IncidentsView {}

impl IncidentsView {
    pub fn new(connection: CamundaConnection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(tr!("incidents.query.placeholder")));
        let definition = cx.new(|cx| SelectState::new(vec![DefinitionOption::all()], Some(IndexPath::default()), window, cx));
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
            selected_group: None,
            table,
            notice: None,
            error: None,
            load: None,
            locale: locale::current(),
            _subscriptions: subscriptions,
        };
        view.reload(window, cx);
        view
    }
}

impl IncidentsView {
    // После смены языка обновляет тексты, которые компоненты GPUI Kit держат у себя.
    fn relabel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = locale::current();
        if current == self.locale {
            return;
        }
        self.locale = current;
        self.query.update(cx, |state, cx| state.set_placeholder(tr!("incidents.query.placeholder"), window, cx));
        let options = self.definition_options();
        self.definition.update(cx, |state, cx| state.set_items(options, window, cx));
        self.table.update(cx, |table, cx| table.refresh(cx));
    }
}

impl Render for IncidentsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.relabel(window, cx);
        let filtered = self.filtered(cx);
        let body = if self.load.is_some() && self.incidents.is_empty() {
            load::skeleton().into_any_element()
        } else if filtered.is_empty() {
            load::empty(self.incidents.is_empty()).into_any_element()
        } else {
            h_resizable("incident-panes")
                .child(resizable_panel().size(px(380.)).size_range(px(260.)..px(720.)).pr_2().child(self.render_groups(&filtered, cx)))
                .child(resizable_panel().pl_2().child(self.render_list(cx)))
                .into_any_element()
        };
        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(self.render_header(cx))
            .child(self.render_filters(cx))
            .when_some(self.notice.clone(), |this, notice| this.child(Alert::success("incidents-notice", notice)))
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("incidents-error", error)))
            .child(div().flex_1().min_h_0().child(body))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn plural_forms_resolve_in_both_languages() {
        assert_eq!(rust_i18n::t!("definition.instances.other", locale = "en", count = 3), "3 instances");
        assert_eq!(rust_i18n::t!("definition.instances.few", locale = "ru", count = 3), "3 заявки");
    }
}
