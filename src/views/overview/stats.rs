use camunda_core::camunda::dates;
use camunda_core::models::{DashboardDatePreset, ProcessDashboard, ProcessIncident};
use chrono::{Local, Timelike};
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::chart::AreaChart;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{OverviewView, STUCK_MILLIS};
use crate::model;
use crate::platform::motion;

const HOURS: usize = 24;

struct Stat {
    id: &'static str,
    label: SharedString,
    caption: SharedString,
    value: i64,
    icon: Lucide,
    tone: Option<Hsla>,
}

impl OverviewView {
    // Карточки сводки: число набегает при загрузке, у инцидентов график по часам.
    pub(super) fn render_stats(&self, dashboard: &ProcessDashboard, cx: &App) -> impl IntoElement {
        let incidents = self.incident_groups.iter().map(|group| group.count).sum::<usize>() as i64;
        let instances: i64 = dashboard.definitions.iter().map(|d| d.instances as i64).sum();
        let stuck: i64 = dashboard
            .longest_waiting_activities
            .iter()
            .filter(|item| item.maximum_waiting_millis >= STUCK_MILLIS)
            .map(|item| item.instance_count as i64)
            .sum();
        let live = self.preset == DashboardDatePreset::Live;
        let theme = cx.theme();
        let stats = [
            Stat {
                id: "stat-incidents",
                label: tr!(if live { "overview.stats.incidents_24h" } else { "overview.stats.incidents" }),
                caption: tr!("overview.stats.incidents_caption", groups = self.incident_groups.len()),
                value: incidents,
                icon: Lucide::TriangleAlert,
                tone: (incidents > 0).then_some(theme.danger),
            },
            Stat {
                id: "stat-running",
                label: tr!(if live { "overview.stats.running" } else { "overview.stats.started" }),
                caption: tr!("overview.stats.running_caption", processes = self.processes.len()),
                value: instances,
                icon: Lucide::Play,
                tone: None,
            },
            Stat {
                id: "stat-stuck",
                label: tr!("overview.stats.stuck"),
                caption: tr!("overview.stats.stuck_caption", activities = dashboard.longest_waiting_activities.len()),
                value: stuck,
                icon: Lucide::CircleAlert,
                tone: (stuck > 0).then_some(theme.warning),
            },
            Stat {
                id: "stat-completed",
                label: tr!("overview.stats.completed"),
                caption: tr!("overview.stats.completed_caption"),
                value: dashboard.completed_instances as i64,
                icon: Lucide::CircleCheck,
                tone: None,
            },
        ];
        let trend = hourly(&dashboard.incidents, self.preset);
        h_flex().w_full().gap_3().children(stats.into_iter().enumerate().map(|(index, stat)| {
            let sparkline = (index == 0 && incidents > 0).then(|| sparkline(trend.clone(), self.preset, cx));
            motion::stagger(SharedString::from(format!("{}-card", stat.id)), index, card(stat, sparkline, cx))
        }))
    }
}

fn card(stat: Stat, sparkline: Option<AnyElement>, cx: &App) -> Div {
    let theme = cx.theme();
    let accent = stat.tone.unwrap_or(theme.muted_foreground);
    v_flex()
        .flex_1()
        .min_w_0()
        .h(px(118.))
        .p_3()
        .gap_1()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(stat.tone.map(|tone| tone.opacity(0.35)).unwrap_or(theme.border))
        .bg(stat.tone.map(|tone| tone.opacity(0.06)).unwrap_or(theme.secondary.opacity(0.4)))
        .overflow_hidden()
        .relative()
        .child(
            h_flex()
                .gap_2()
                .child(
                    div()
                        .size(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(theme.radius)
                        .bg(accent.opacity(0.14))
                        .child(Icon::new(stat.icon).small().text_color(accent)),
                )
                .child(div().truncate().text_sm().text_color(theme.muted_foreground).child(stat.label)),
        )
        .child(
            h_flex()
                .items_end()
                .gap_3()
                .child(
                    div()
                        .text_size(px(30.))
                        .line_height(px(36.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .when_some(stat.tone, |this, tone| this.text_color(tone))
                        .child(motion::count_up((stat.id, stat.value as u64), stat.value, model::grouped)),
                )
                .when_some(sparkline, |this, chart| this.child(div().flex_1().min_w_0().h(px(40.)).child(chart))),
        )
        .child(div().truncate().text_xs().text_color(theme.muted_foreground).child(stat.caption))
}

fn sparkline(trend: Vec<(SharedString, f64)>, preset: DashboardDatePreset, cx: &App) -> AnyElement {
    let danger = cx.theme().danger;
    let peak = trend.iter().map(|(_, count)| *count).fold(1., f64::max);
    AreaChart::new(trend)
        .id(("incident-trend", preset as usize))
        .x(|(hour, _)| hour.clone())
        .y(|(_, count)| *count)
        .natural()
        .stroke(danger)
        .fill(linear_gradient(180., linear_color_stop(danger.opacity(0.35), 0.), linear_color_stop(danger.opacity(0.), 1.)))
        .x_axis(false)
        .y_axis(false)
        .grid(false)
        .y_domain(0., peak)
        .appear(true)
        .tooltip_value(|_, _, count| tr!("overview.stats.per_hour", count = count as i64))
        .into_any_element()
}

// Инциденты по часам: в режиме «Сейчас» за последние 24 часа, иначе по часам выбранного дня.
fn hourly(incidents: &[ProcessIncident], preset: DashboardDatePreset) -> Vec<(SharedString, f64)> {
    let now = Local::now();
    let mut buckets = [0usize; HOURS];
    for incident in incidents {
        let Some(at) = dates::parse_camunda_date_time(incident.timestamp.as_deref()).map(|at| at.with_timezone(&Local)) else { continue };
        let slot = if preset == DashboardDatePreset::Live {
            let ago = (now - at).num_hours();
            if !(0..HOURS as i64).contains(&ago) {
                continue;
            }
            HOURS - 1 - ago as usize
        } else {
            at.hour() as usize
        };
        buckets[slot] += 1;
    }
    buckets
        .iter()
        .enumerate()
        .map(|(slot, count)| {
            let hour = if preset == DashboardDatePreset::Live { (now.hour() as usize + slot + 1) % HOURS } else { slot };
            (SharedString::from(format!("{hour:02}:00")), *count as f64)
        })
        .collect()
}
