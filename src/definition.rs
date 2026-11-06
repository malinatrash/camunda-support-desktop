use camunda_core::models::{
    CamundaConnection, DashboardDateFilter, ProcessDefinitionDetails, ProcessDefinitionSummary, RuntimeInstanceListItem,
};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme, IconName, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model;
use crate::services;
use crate::ui::{self, Navigate, Tone};

pub struct DefinitionView {
    connection: CamundaConnection,
    definition_id: String,
    version_id: Option<String>,
    versions: Vec<ProcessDefinitionSummary>,
    details: Option<ProcessDefinitionDetails>,
    error: Option<String>,
    table: Entity<TableState<InstanceTable>>,
    load: Option<Task<()>>,
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
            _versions_load: versions_load,
            _subscriptions: vec![subscription],
        };
        view.reload(cx);
        view
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let id = self.version_id.clone().unwrap_or_else(|| self.definition_id.clone());
        let all_versions = self.version_id.is_none();
        let request = services::request(cx, move |api| async move {
            api.load_process_definition_details(&connection, &id, &DashboardDateFilter::default(), all_versions).await
        });
        self.load = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(details) => {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().rows = details.instances.clone();
                            table.refresh(cx);
                        });
                        this.details = Some(details);
                        this.error = None;
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

    fn select_version(&mut self, version_id: Option<String>, cx: &mut Context<Self>) {
        self.version_id = version_id;
        self.reload(cx);
    }

    fn render_versions(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let all = Button::new("all-versions")
            .label("Все версии")
            .small()
            .map(|button| if self.version_id.is_none() { button.secondary() } else { button.ghost() })
            .on_click(cx.listener(|this, _, _, cx| this.select_version(None, cx)));
        h_flex().id("versions").gap_1().overflow_x_scroll().child(all).children(self.versions.iter().map(|version| {
            let id = version.id.clone();
            let selected = self.version_id.as_deref() == Some(version.id.as_str());
            Button::new(SharedString::from(version.id.clone()))
                .label(format!("v{}", version.version))
                .small()
                .map(|button| if selected { button.secondary() } else { button.ghost() })
                .on_click(cx.listener(move |this, _, _, cx| this.select_version(Some(id.clone()), cx)))
        }))
    }
}

impl Render for DefinitionView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key = self.definition_id.split(':').next().unwrap_or_default().to_string();
        let title =
            self.details.as_ref().map(|details| model::display_name(&details.definition).to_string()).unwrap_or_else(|| key.clone());
        let rows = self.details.as_ref().map(|details| details.instances.len()).unwrap_or_default();
        let with_incidents = self
            .details
            .as_ref()
            .map(|details| details.instances.iter().filter(|item| item.incident_count > 0).count())
            .unwrap_or_default();
        v_flex()
            .size_full()
            .p_5()
            .gap_4()
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Button::new("back")
                            .icon(IconName::ChevronLeft)
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Navigate::Back))),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().text_lg().font_weight(FontWeight::SEMIBOLD).child(title))
                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(key)),
                    )
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!("{rows} заявок")))
                    .when(with_incidents > 0, |this| this.child(ui::pill(format!("{with_incidents} с инцидентами"), Tone::Danger, cx)))
                    .child(
                        Button::new("reload")
                            .icon(IconName::RefreshCw)
                            .ghost()
                            .small()
                            .loading(self.load.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
                    ),
            )
            .child(self.render_versions(cx))
            .when_some(self.error.clone(), |this, error| {
                this.child(div().px_3().py_2().rounded_md().bg(cx.theme().danger.opacity(0.1)).text_color(cx.theme().danger).child(error))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_md()
                    .overflow_hidden()
                    .child(DataTable::new(&self.table).large().stripe(false).bordered(false)),
            )
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Двойной клик по строке открывает заявку"))
    }
}

pub struct InstanceTable {
    rows: Vec<RuntimeInstanceListItem>,
    columns: Vec<Column>,
}

impl Default for InstanceTable {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            columns: vec![
                Column::new("state", "Состояние").width(140.),
                Column::new("business_key", "Бизнес-ключ").width(220.),
                Column::new("id", "ID заявки").width(320.),
                Column::new("start", "Создана").width(160.),
                Column::new("version", "Версия").width(80.),
                Column::new("incidents", "Инциденты").width(100.),
                Column::new("tenant", "Тенант").width(140.),
            ],
        }
    }
}

impl TableDelegate for InstanceTable {
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
        let item = &self.rows[row_ix];
        let instance = &item.instance;
        match self.columns[col_ix].key.as_ref() {
            "state" => ui::instance_state(instance.suspended, instance.ended, cx).into_any_element(),
            "business_key" => div().truncate().child(instance.business_key.clone().unwrap_or_else(|| "—".into())).into_any_element(),
            "id" => div().truncate().text_sm().font_family(crate::theme::MONO).child(instance.id.clone()).into_any_element(),
            "start" => div()
                .text_sm()
                .font_family(crate::theme::MONO)
                .text_color(cx.theme().muted_foreground)
                .child(model::short_time(instance.start_time.as_deref()))
                .into_any_element(),
            "version" => {
                let version = instance.definition_id.split(':').nth(1).unwrap_or("?");
                div().text_sm().text_color(cx.theme().muted_foreground).child(format!("v{version}")).into_any_element()
            }
            "incidents" if item.incident_count > 0 => ui::pill(item.incident_count.to_string(), Tone::Danger, cx).into_any_element(),
            "incidents" => div().text_color(cx.theme().muted_foreground).child("0").into_any_element(),
            _ => div()
                .truncate()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(instance.tenant_id.clone().unwrap_or_else(|| "—".into()))
                .into_any_element(),
        }
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<SharedString> {
        let instance = &self.rows.get(row_ix)?.instance;
        match self.columns[col_ix].key.as_ref() {
            "business_key" => instance.business_key.clone().map(Into::into),
            "id" => Some(instance.id.clone().into()),
            _ => None,
        }
    }
}
