use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme, Sizable, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::TeleportView;

impl TeleportView {
    // Колонка «откуда»: активные экземпляры, которые можно отменить.
    pub(super) fn render_sources(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.source_query.read(cx).value().to_string();
        let rows: Vec<AnyElement> = self
            .sources
            .iter()
            .filter(|a| matches(&query, &[a.activity_name.as_deref(), Some(&a.activity_id), Some(&a.activity_type), Some(&a.id)]))
            .map(|activity| {
                let id = activity.id.clone();
                let title = activity.activity_name.clone().unwrap_or_else(|| activity.activity_id.clone());
                let meta = format!("{} · {}", activity.activity_type, activity.activity_id);
                option_row(
                    SharedString::from(format!("source-{}", activity.id)),
                    self.source.as_deref() == Some(activity.id.as_str()),
                    title,
                    meta,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.source = Some(id.clone());
                    cx.notify();
                }))
                .into_any_element()
            })
            .collect();
        picker("sources", tr!("instance.teleport.step_from"), tr!("instance.teleport.source_title"), &self.source_query, rows, cx)
    }

    pub(super) fn render_targets(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.target_query.read(cx).value().to_string();
        let rows: Vec<AnyElement> = self
            .targets
            .iter()
            .filter(|n| matches(&query, &[n.name.as_deref(), Some(&n.id), Some(&n.node_type), n.topic.as_deref()]))
            .map(|node| {
                let id = node.id.clone();
                let title = node.name.clone().unwrap_or_else(|| node.id.clone());
                let topic = node.topic.as_ref().map(|topic| format!(" · {topic}")).unwrap_or_default();
                let meta = format!("{} · {}{topic}", node.node_type, node.id);
                option_row(
                    SharedString::from(format!("target-{}", node.id)),
                    self.target.as_deref() == Some(node.id.as_str()),
                    title,
                    meta,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.target = Some(id.clone());
                    cx.notify();
                }))
                .into_any_element()
            })
            .collect();
        picker("targets", tr!("instance.teleport.step_to"), tr!("instance.teleport.target_title"), &self.target_query, rows, cx)
    }
}

fn picker(
    id: &'static str,
    step: SharedString,
    title: SharedString,
    query: &Entity<InputState>,
    rows: Vec<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .h_full()
        .min_w_0()
        .min_h_0()
        .gap_2()
        .p_4()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(cx.theme().muted_foreground).child(step))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(Input::new(query).small())
        .child(v_flex().id(id).flex_1().min_h_0().gap_1().overflow_y_scroll().children(rows))
}

fn matches(query: &str, values: &[Option<&str>]) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || values.iter().flatten().any(|value| value.to_lowercase().contains(&query))
}

fn option_row(id: SharedString, selected: bool, title: String, meta: String, cx: &App) -> Stateful<Div> {
    let hover = cx.theme().accent;
    v_flex()
        .id(id)
        .px_3()
        .py_2()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(if selected { cx.theme().muted_foreground } else { transparent_black() })
        .when(selected, |this| this.bg(cx.theme().accent))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(div().truncate().font_weight(FontWeight::MEDIUM).child(title))
        .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(meta))
}
