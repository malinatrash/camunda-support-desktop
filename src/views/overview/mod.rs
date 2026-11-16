//! Обзор: сводка, ошибки и ожидание, список процессов с версиями.

mod errors;
mod incidents;
mod load;
mod processes;
mod stats;
mod toolbar;
mod versions;
mod waiting;

use std::time::{Duration, Instant};

use camunda_core::models::{CamundaConnection, DashboardDatePreset, ProcessDashboard};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model::{IncidentGroup, Navigate, ProcessGroup};

const REFRESH_EVERY: Duration = Duration::from_secs(15);
const STUCK_MILLIS: i64 = 4 * 3_600_000;
const TAB_ERRORS: usize = 0;

pub struct OverviewView {
    connection: CamundaConnection,
    preset: DashboardDatePreset,
    dashboard: Option<ProcessDashboard>,
    incident_groups: Vec<IncidentGroup>,
    processes: Vec<ProcessGroup>,
    error: Option<String>,
    refreshed_at: Option<Instant>,
    tab: usize,
    expanded_group: Option<SharedString>,
    expanded_process: Option<SharedString>,
    filter: Entity<InputState>,
    load: Option<Task<()>>,
    _poll: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<Navigate> for OverviewView {}

impl OverviewView {
    pub fn new(connection: CamundaConnection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Фильтр"));
        let subscription = cx.subscribe(&filter, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.regroup(cx);
                cx.notify();
            }
        });
        let poll = cx.spawn(async move |this, cx| {
            loop {
                if this.update(cx, |this, cx| this.refresh(cx)).is_err() {
                    break;
                }
                cx.background_executor().timer(REFRESH_EVERY).await;
            }
        });
        Self {
            connection,
            preset: DashboardDatePreset::Live,
            dashboard: None,
            incident_groups: Vec::new(),
            processes: Vec::new(),
            error: None,
            refreshed_at: None,
            tab: TAB_ERRORS,
            expanded_group: None,
            expanded_process: None,
            filter,
            load: None,
            _poll: poll,
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for OverviewView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match &self.dashboard {
            Some(dashboard) => v_flex()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .gap_4()
                .child(self.render_stats(dashboard, cx))
                .child(
                    div().flex_1().min_w_0().min_h_0().child(
                        h_resizable("overview-split")
                            .child(resizable_panel().size_range(px(320.)..Pixels::MAX).pr_2().child(self.render_errors(dashboard, cx)))
                            .child(resizable_panel().size_range(px(320.)..Pixels::MAX).pl_2().child(self.render_processes(cx))),
                    ),
                )
                .into_any_element(),
            None => v_flex()
                .flex_1()
                .gap_4()
                .child(Skeleton::new().h(px(76.)).rounded(cx.theme().radius))
                .child(Skeleton::new().flex_1().rounded(cx.theme().radius))
                .into_any_element(),
        };
        v_flex()
            .size_full()
            .min_w_0()
            .p_4()
            .gap_4()
            .child(self.render_toolbar(cx))
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("overview-error", error)))
            .child(content)
    }
}
