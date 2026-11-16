use camunda_core::models::ProcessIncident;
use gpui_kit::component::accordion::Accordion;
use gpui_kit::component::button::Button;
use gpui_kit::component::chart::BarChart;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::IncidentsView;
use super::details::incident_card;
use super::retry::retryable;
use crate::model;

impl IncidentsView {
    pub(super) fn render_groups(&self, filtered: &[ProcessIncident], cx: &mut Context<Self>) -> impl IntoElement {
        let groups = model::group_incidents(filtered);
        let keys: Vec<SharedString> = groups.iter().map(|group| group.key.clone()).collect();
        let mut accordion =
            Accordion::new("incident-groups").bordered(false).h_auto().on_toggle_click(cx.listener(move |this, open: &[usize], _, cx| {
                this.selected_group = open.first().and_then(|ix| keys.get(*ix).cloned());
                this.sync_table(cx);
            }));
        for group in groups {
            let members: Vec<&ProcessIncident> = filtered.iter().filter(|incident| group_key(incident) == group.key.as_ref()).collect();
            let retry: Vec<ProcessIncident> =
                members.iter().filter(|incident| retryable(incident)).map(|incident| (*incident).clone()).collect();
            let meta = tr!(
                "incidents.group_meta",
                process = group.process_key,
                kind = model::incident_type_label(&group.incident_type),
                count = group.instance_ids.len(),
                time = model::short_time(group.latest.as_deref())
            );
            let title = h_flex().gap_3().min_w_0().child(Tag::danger().small().rounded_full().child(group.count.to_string())).child(
                v_flex()
                    .min_w_0()
                    .child(div().truncate().child(group.activity.clone()))
                    .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(group.signature.clone())),
            );
            let actions =
                h_flex().gap_2().child(div().flex_1().truncate().text_xs().text_color(cx.theme().muted_foreground).child(meta)).child(
                    Button::new(SharedString::from(format!("retry-{}", group.key)))
                        .label(tr!("incidents.retry_group", count = retry.len()))
                        .outline()
                        .xsmall()
                        .disabled(retry.is_empty())
                        .on_click(cx.listener(move |this, _, window, cx| this.ask_retry(retry.clone(), window, cx))),
                );
            let title = match members.first() {
                Some(sample) => incident_card(format!("group-card-{}", group.key), sample, title).into_any_element(),
                None => title.into_any_element(),
            };
            let open = self.selected_group.as_ref() == Some(&group.key);
            accordion = accordion.item(|item| item.open(open).title(title).child(actions));
        }
        v_flex()
            .size_full()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(tr!("incidents.groups")),
            )
            .child(div().id("incident-groups-scroll").flex_1().min_h_0().overflow_y_scroll().child(accordion))
            .child(by_process(filtered, cx))
    }
}

// Столбики инцидентов по процессам под списком групп.
fn by_process(incidents: &[ProcessIncident], cx: &App) -> impl IntoElement {
    let mut counts: Vec<(SharedString, f64)> = Vec::new();
    for incident in incidents {
        let process = incident.process_definition_id.as_deref().and_then(|id| id.split(':').next()).unwrap_or_default();
        match counts.iter_mut().find(|(key, _)| key.as_ref() == process) {
            Some((_, count)) => *count += 1.,
            None => counts.push((SharedString::from(process.to_string()), 1.)),
        }
    }
    counts.sort_by(|a, b| b.1.total_cmp(&a.1));
    counts.truncate(6);
    let danger = cx.theme().danger;
    v_flex()
        .px_4()
        .py_3()
        .gap_2()
        .border_t_1()
        .border_color(cx.theme().border)
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(tr!("incidents.by_process")))
        .child(
            div().h(px(150.)).child(
                BarChart::new(counts)
                    .band(|(process, _)| process.clone())
                    .value(|(_, count)| *count)
                    .fill(move |_, _, _, _| danger.opacity(0.75))
                    .label(|(_, count)| (*count as i64).to_string())
                    .corner_radii(px(4.))
                    .max_band_width(px(48.))
                    .value_axis(false)
                    .grid(false)
                    .appear(true)
                    .tooltip_value(|_, count| tr!("incidents.count", count = count as i64)),
            ),
        )
}

pub(super) fn group_key(incident: &ProcessIncident) -> String {
    let activity = incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()).unwrap_or_default();
    let process_key = incident.process_definition_id.as_deref().and_then(|id| id.split(':').next()).unwrap_or_default();
    format!("{process_key}|{activity}|{}|{}", incident.incident_type, model::incident_signature(incident.message.as_deref()))
}
