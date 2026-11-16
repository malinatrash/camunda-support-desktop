mod actions;
mod activities;
mod diagram;
mod header;
mod hover;
mod metadata;
mod operations;
mod panel;
mod report;
mod teleport;
mod timeline;
mod variables;

use std::time::Duration;

use camunda_core::models::{CamundaConnection, ProcessInstanceDetails};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::{ActiveTheme, WindowExt as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::bpmn::BpmnView;
use crate::model::Navigate;
use crate::platform::services;
use teleport::TeleportView;
use variables::VariablesView;

const TABS: [&str; 5] = ["Схема", "Хронология", "Диагностика", "Переменные", "Метаданные"];
const REFRESH_EVERY: Duration = Duration::from_secs(30);

pub struct InstanceView {
    connection: CamundaConnection,
    instance_id: String,
    details: Option<ProcessInstanceDetails>,
    diagram: Option<Entity<BpmnView>>,
    variables: Option<Entity<VariablesView>>,
    teleport: Option<Entity<TeleportView>>,
    operation: Option<Task<()>>,
    selected_node: Option<String>,
    tab: usize,
    notice: Option<SharedString>,
    error: Option<String>,
    load: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
    _teleport_subscription: Option<Subscription>,
    _poll: Option<Task<()>>,
}

impl EventEmitter<Navigate> for InstanceView {}

impl InstanceView {
    pub fn new(connection: CamundaConnection, instance_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            connection,
            instance_id,
            details: None,
            diagram: None,
            variables: None,
            teleport: None,
            operation: None,
            selected_node: None,
            tab: 0,
            notice: None,
            error: None,
            load: None,
            _subscriptions: Vec::new(),
            _teleport_subscription: None,
            _poll: None,
        };
        view.reload(window, cx);
        view._poll = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH_EVERY).await;
                let alive = this.update_in(cx, |this, window, cx| {
                    if this.load.is_none() && !window.has_active_dialog(cx) {
                        this.reload(window, cx);
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        }));
        view
    }

    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let id = self.instance_id.clone();
        let request = services::request(cx, move |api| async move { api.load_process_instance_details(&connection, &id).await });
        self.load = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(details) => this.show(details, window, cx),
                    Err(error) => this.error = Some(error),
                }
                this.load = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }
}

impl Render for InstanceView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match (&self.details, &self.diagram) {
            (Some(details), Some(diagram)) => match self.tab {
                0 => self.render_diagram(details, diagram, cx).into_any_element(),
                1 => timeline::render(details, cx).into_any_element(),
                2 => operations::render_diagnostics(details, self.operation.is_some(), cx).into_any_element(),
                3 => self.variables.clone().map(|view| view.into_any_element()).unwrap_or_else(|| div().into_any_element()),
                _ => metadata::render(details, cx).into_any_element(),
            },
            _ => v_flex()
                .flex_1()
                .gap_3()
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Загружаем заявку…"))
                .child(Skeleton::new().w_full().flex_1().rounded(cx.theme().radius))
                .into_any_element(),
        };
        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(self.render_header(cx))
            .child(TabBar::new("instance-tabs").underline().selected_index(self.tab).children(TABS).on_click(cx.listener(
                |this, index: &usize, _, cx| {
                    this.tab = *index;
                    cx.notify();
                },
            )))
            .when_some(self.notice.clone(), |this, notice| this.child(Alert::success("instance-notice", notice)))
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("instance-error", error)))
            .child(v_flex().flex_1().min_h_0().child(body))
    }
}
