use std::time::{Duration, Instant};

use camunda_core::models::{CamundaConnection, DashboardDateFilter, DashboardDatePreset, ProcessDashboard};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::model::{self, IncidentGroup, ProcessGroup};
use crate::services;
use crate::ui::{self, Navigate, Tone};

const REFRESH_EVERY: Duration = Duration::from_secs(15);
const STUCK_MILLIS: i64 = 4 * 3_600_000;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Errors,
    Waiting,
}

pub struct OverviewView {
    connection: CamundaConnection,
    preset: DashboardDatePreset,
    dashboard: Option<ProcessDashboard>,
    incident_groups: Vec<IncidentGroup>,
    processes: Vec<ProcessGroup>,
    error: Option<String>,
    refreshed_at: Option<Instant>,
    tab: Tab,
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
            tab: Tab::Errors,
            expanded_group: None,
            expanded_process: None,
            filter,
            load: None,
            _poll: poll,
            _subscriptions: vec![subscription],
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let connection = self.connection.clone();
        let filter = DashboardDateFilter { preset: self.preset, ..Default::default() };
        let request = services::request(cx, move |api| async move { api.load_dashboard(&connection, &filter).await });
        self.load = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(dashboard) => {
                        this.incident_groups = model::group_incidents(&dashboard.incidents);
                        this.dashboard = Some(dashboard);
                        this.error = None;
                        this.refreshed_at = Some(Instant::now());
                        this.regroup(cx);
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

    fn regroup(&mut self, cx: &App) {
        let Some(dashboard) = &self.dashboard else { return };
        let needle = self.filter.read(cx).value().trim().to_lowercase();
        let definitions: Vec<_> = dashboard
            .definitions
            .iter()
            .filter(|d| {
                needle.is_empty()
                    || d.key.to_lowercase().contains(&needle)
                    || d.name.as_deref().is_some_and(|name| name.to_lowercase().contains(&needle))
            })
            .cloned()
            .collect();
        self.processes = model::group_processes(&definitions);
    }

    fn select_preset(&mut self, preset: DashboardDatePreset, cx: &mut Context<Self>) {
        self.preset = preset;
        self.dashboard = None;
        self.refresh(cx);
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let presets = [
            (DashboardDatePreset::Live, "Сейчас"),
            (DashboardDatePreset::Today, "Сегодня"),
            (DashboardDatePreset::Yesterday, "Вчера"),
        ];
        let status: SharedString = if self.load.is_some() && self.dashboard.is_none() {
            "загружаем…".into()
        } else {
            self.refreshed_at.map(|at| format!("обновлено {} с назад", at.elapsed().as_secs())).unwrap_or_default().into()
        };
        h_flex()
            .gap_1()
            .children(presets.into_iter().map(|(preset, label)| {
                Button::new(label)
                    .label(label)
                    .small()
                    .map(|button| if self.preset == preset { button.secondary() } else { button.ghost() })
                    .on_click(cx.listener(move |this, _, _, cx| this.select_preset(preset, cx)))
            }))
            .child(div().flex_1())
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(status))
            .child(
                Button::new("refresh")
                    .icon(IconName::RefreshCw)
                    .ghost()
                    .small()
                    .loading(self.load.is_some())
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
            )
    }

    fn render_stats(&self, dashboard: &ProcessDashboard, cx: &App) -> impl IntoElement {
        let incidents: i32 = dashboard.definitions.iter().map(|d| d.incidents).sum();
        let instances: i32 = dashboard.definitions.iter().map(|d| d.instances).sum();
        let stuck: i32 = dashboard
            .longest_waiting_activities
            .iter()
            .filter(|item| item.maximum_waiting_millis >= STUCK_MILLIS)
            .map(|item| item.instance_count)
            .sum();
        let live = self.preset == DashboardDatePreset::Live;
        let stat = |value: i32, label: &'static str, color: Option<Hsla>| {
            h_flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(26.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .when_some(color, |this, color| this.text_color(color))
                        .child(model::grouped(value as i64)),
                )
                .child(div().text_color(cx.theme().muted_foreground).child(label))
        };
        h_flex()
            .flex_wrap()
            .gap_x(px(40.))
            .gap_y_2()
            .px_4()
            .py_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .bg(cx.theme().table_head)
            .child(stat(incidents, "инцидентов", (incidents > 0).then_some(cx.theme().danger)))
            .child(stat(instances, if live { "в работе" } else { "стартов" }, None))
            .child(stat(stuck, "висят дольше 4 ч", (stuck > 0).then_some(cx.theme().warning)))
            .child(stat(dashboard.completed_instances, "завершено", None))
    }

    fn render_errors(&self, dashboard: &ProcessDashboard, cx: &mut Context<Self>) -> impl IntoElement {
        let head = h_flex()
            .gap_4()
            .h_full()
            .child(self.tab_button(Tab::Errors, "Ошибки", self.incident_groups.len(), cx))
            .child(self.tab_button(Tab::Waiting, "Ожидание", dashboard.longest_waiting_activities.len(), cx));
        let body = div().id("errors").flex_1().overflow_y_scroll();
        let body = match self.tab {
            Tab::Errors if self.incident_groups.is_empty() => body.child(div().p_6().text_color(cx.theme().muted_foreground).child("Инцидентов нет")),
            Tab::Errors => body.children(self.incident_groups.iter().map(|group| self.render_group(group, cx))),
            Tab::Waiting => body.children(dashboard.longest_waiting_activities.iter().map(|item| {
                let id = item.oldest_process_instance_id.clone();
                let tone = if item.maximum_waiting_millis >= 24 * 3_600_000 {
                    Tone::Danger
                } else if item.maximum_waiting_millis >= STUCK_MILLIS {
                    Tone::Warning
                } else {
                    Tone::Neutral
                };
                h_flex()
                    .id(SharedString::from(format!("wait-{}-{}", item.process_definition_id, item.activity_id)))
                    .px_4()
                    .py_3()
                    .gap_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .hover(|style| style.bg(cx.theme().table_hover))
                    .cursor_pointer()
                    .child(div().w(px(110.)).flex_none().child(ui::pill(model::format_duration(item.maximum_waiting_millis), tone, cx)))
                    .child(
                        v_flex()
                            .min_w_0()
                            .child(div().truncate().font_weight(FontWeight::MEDIUM).child(item.activity_name.clone().unwrap_or_else(|| item.activity_id.clone())))
                            .child(
                                div()
                                    .truncate()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{} · заявок {}", item.process_definition_id.split(':').next().unwrap_or_default(), item.instance_count)),
                            ),
                    )
                    .on_click(cx.listener(move |_, _, _, cx| cx.emit(Navigate::Instance(id.clone()))))
            })),
        };
        ui::section(head, cx).flex_1().child(body)
    }

    fn tab_button(&self, tab: Tab, label: &'static str, count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.tab == tab;
        h_flex()
            .id(label)
            .h_full()
            .gap_1p5()
            .border_b_2()
            .border_color(if active { cx.theme().primary } else { transparent_black() })
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(if active { cx.theme().foreground } else { cx.theme().muted_foreground })
            .cursor_pointer()
            .child(label)
            .child(ui::counter(count, cx))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.tab = tab;
                cx.notify();
            }))
    }

    fn render_group(&self, group: &IncidentGroup, cx: &mut Context<Self>) -> AnyElement {
        let open = self.expanded_group.as_ref() == Some(&group.key);
        let key = group.key.clone();
        let meta = format!(
            "{} · {} · заявок {} · {}",
            group.process_key,
            model::incident_type_label(&group.incident_type),
            group.instance_ids.len(),
            model::short_time(group.latest.as_deref())
        );
        v_flex()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(open, |this| this.bg(cx.theme().table_head))
            .child(
                h_flex()
                    .id(group.key.clone())
                    .items_start()
                    .px_4()
                    .py_3()
                    .gap_3()
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.theme().table_hover))
                    .child(div().w(px(44.)).flex_none().flex().justify_end().child(ui::pill(group.count.to_string(), Tone::Danger, cx)))
                    .child(
                        v_flex()
                            .min_w_0()
                            .child(div().truncate().font_weight(FontWeight::MEDIUM).child(group.activity.clone()))
                            .child(div().truncate().text_sm().child(group.signature.clone()))
                            .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(meta)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.expanded_group = if this.expanded_group.as_ref() == Some(&key) { None } else { Some(key.clone()) };
                        cx.notify();
                    })),
            )
            .when(open, |this| {
                this.child(v_flex().pl(px(72.)).pb_3().children(group.instance_ids.iter().take(50).map(|id| {
                    let target = id.clone();
                    div()
                        .id(SharedString::from(format!("{}-{id}", group.key)))
                        .py_1()
                        .text_sm()
                        .text_color(cx.theme().link)
                        .cursor_pointer()
                        .hover(|style| style.underline())
                        .child(id.clone())
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(Navigate::Instance(target.clone()))))
                })))
            })
            .into_any_element()
    }

    fn render_processes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let live = self.preset == DashboardDatePreset::Live;
        let head = h_flex()
            .w_full()
            .gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Процессы"))
            .child(ui::counter(self.processes.len(), cx))
            .child(div().flex_1())
            .child(div().w(px(170.)).child(Input::new(&self.filter).small()));
        let columns = h_flex()
            .flex_none()
            .px_4()
            .py_2()
            .gap_3()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(cx.theme().muted_foreground)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().flex_1().pl(px(28.)).child("Процесс"))
            .child(div().w(px(80.)).text_right().child(if live { "в работе" } else { "стартов" }))
            .child(div().w(px(80.)).text_right().child("инциденты"));
        ui::section(head, cx)
            .w(px(460.))
            .flex_none()
            .child(columns)
            .child(div().id("processes").flex_1().overflow_y_scroll().children(self.processes.iter().map(|group| self.render_process(group, cx))))
    }

    fn render_process(&self, group: &ProcessGroup, cx: &mut Context<Self>) -> AnyElement {
        let open = self.expanded_process.as_ref() == Some(&group.key);
        let key = group.key.clone();
        let latest = &group.latest;
        let versions = if group.versions.len() > 1 { format!(" · версий {}", group.versions.len()) } else { String::new() };
        let all_id = latest.id.clone();
        v_flex()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(open, |this| this.bg(cx.theme().table_head))
            .child(
                h_flex()
                    .id(group.key.clone())
                    .px_4()
                    .py_3()
                    .gap_3()
                    .cursor_pointer()
                    .hover(|style| style.bg(cx.theme().table_hover))
                    .child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).small().text_color(cx.theme().muted_foreground))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().child(model::display_name(latest).to_string()))
                            .child(
                                div()
                                    .truncate()
                                    .text_xs()
                                    .font_family(crate::theme::MONO)
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{} · v{}{versions}{}", latest.key, latest.version, if latest.suspended { " · пауза" } else { "" })),
                            ),
                    )
                    .child(div().w(px(80.)).text_right().child(model::grouped(group.instances as i64)))
                    .child(div().w(px(80.)).flex().justify_end().child(incidents_cell(group.incidents, cx)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.expanded_process = if this.expanded_process.as_ref() == Some(&key) { None } else { Some(key.clone()) };
                        cx.notify();
                    })),
            )
            .when(open, |this| {
                this.child(version_row(
                    SharedString::from(format!("{}-all", group.key)),
                    "Все версии".into(),
                    None,
                    group.instances,
                    None,
                    cx,
                    cx.listener(move |_, _, _, cx| cx.emit(Navigate::Definition { id: all_id.clone(), all_versions: true })),
                ))
                .children(group.versions.iter().map(|version| {
                    let id = version.id.clone();
                    let meta = format!(
                        "{}{}{}",
                        model::short_time(version.deployment_time.as_deref()),
                        version.version_tag.as_ref().map(|tag| format!(" · {tag}")).unwrap_or_default(),
                        if version.suspended { " · пауза" } else { "" }
                    );
                    version_row(
                        SharedString::from(version.id.clone()),
                        format!("v{}", version.version).into(),
                        Some(meta.into()),
                        version.instances,
                        Some(version.incidents),
                        cx,
                        cx.listener(move |_, _, _, cx| cx.emit(Navigate::Definition { id: id.clone(), all_versions: false })),
                    )
                }))
            })
            .into_any_element()
    }
}

fn incidents_cell(count: i32, cx: &App) -> AnyElement {
    if count > 0 {
        ui::pill(count.to_string(), Tone::Danger, cx).into_any_element()
    } else {
        div().text_color(cx.theme().muted_foreground).child("0").into_any_element()
    }
}

fn version_row(
    id: SharedString,
    title: SharedString,
    meta: Option<SharedString>,
    instances: i32,
    incidents: Option<i32>,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    h_flex()
        .id(id)
        .pl(px(44.))
        .pr_4()
        .py_2()
        .gap_3()
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().table_hover).text_color(cx.theme().link))
        .child(div().w(px(90.)).flex_none().font_weight(FontWeight::MEDIUM).child(title))
        .child(div().flex_1().min_w_0().truncate().text_color(cx.theme().muted_foreground).children(meta))
        .child(div().w(px(80.)).text_right().child(model::grouped(instances as i64)))
        .child(div().w(px(80.)).flex().justify_end().children(incidents.map(|count| incidents_cell(count, cx))))
        .on_click(on_click)
}

impl Render for OverviewView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.dashboard.clone() {
            Some(dashboard) => v_flex()
                .flex_1()
                .min_h_0()
                .gap_4()
                .child(self.render_stats(&dashboard, cx))
                .child(h_flex().flex_1().min_h_0().items_start().gap_4().child(self.render_errors(&dashboard, cx)).child(self.render_processes(cx)))
                .into_any_element(),
            None => div().p_6().text_color(cx.theme().muted_foreground).child("Загружаем дашборд…").into_any_element(),
        };
        v_flex()
            .size_full()
            .p_5()
            .gap_4()
            .child(self.render_toolbar(cx))
            .when_some(self.error.clone(), |this, error| {
                this.child(div().px_3().py_2().rounded_md().bg(cx.theme().danger.opacity(0.1)).text_color(cx.theme().danger).child(error))
            })
            .child(content)
    }
}
