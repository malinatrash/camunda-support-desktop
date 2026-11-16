//! Страница процесса: версии и список заявок.

mod header;
mod load;
mod table;
mod versions;

use camunda_core::models::{CamundaConnection, ProcessDefinitionDetails, ProcessDefinitionSummary};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::table::{DataTable, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme, Sizable, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model::Navigate;
use crate::platform::locale;
use crate::platform::services;
use table::InstanceTable;

pub struct DefinitionView {
    connection: CamundaConnection,
    definition_id: String,
    version_id: Option<String>,
    versions: Vec<ProcessDefinitionSummary>,
    details: Option<ProcessDefinitionDetails>,
    error: Option<String>,
    table: Entity<TableState<InstanceTable>>,
    load: Option<Task<()>>,
    locale: SharedString,
    _versions_load: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for DefinitionView {}

impl DefinitionView {
    pub fn new(
        connection: CamundaConnection,
        definition_id: String,
        all_versions: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let table = cx.new(|cx| TableState::new(InstanceTable::default(), window, cx));
        let subscription = cx.subscribe(&table, |_, table, event: &TableEvent, cx| {
            if let TableEvent::DoubleClickedRow(row) = event
                && let Some(item) = table.read(cx).delegate().rows.get(*row)
                && !item.instance.ended
            {
                cx.emit(Navigate::Instance(item.instance.id.clone()));
            }
        });
        let key = definition_id.split(':').next().unwrap_or_default().to_string();
        let versions_connection = connection.clone();
        let request =
            services::request(cx, move |api| async move { api.load_process_definition_versions(&versions_connection, &key).await });
        let versions_load = cx.spawn(async move |this, cx| {
            if let Ok(versions) = request.await {
                this.update(cx, |this, cx| {
                    this.versions = versions;
                    cx.notify();
                })
                .ok();
            }
        });
        let mut view = Self {
            connection,
            version_id: (!all_versions).then(|| definition_id.clone()),
            definition_id,
            versions: Vec::new(),
            details: None,
            error: None,
            table,
            load: None,
            locale: locale::current(),
            _versions_load: versions_load,
            _subscriptions: vec![subscription],
        };
        view.reload(cx);
        view
    }
}

impl Render for DefinitionView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current = locale::current();
        if current != self.locale {
            self.locale = current;
            self.table.update(cx, |table, cx| table.refresh(cx));
        }
        v_flex()
            .size_full()
            .min_w_0()
            .p_4()
            .gap_4()
            .child(self.render_header(cx))
            .child(self.render_versions(cx))
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("definition-error", error)))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .overflow_hidden()
                    .child(DataTable::new(&self.table).large().stripe(false).bordered(false)),
            )
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(tr!("definition.double_click")))
    }
}
