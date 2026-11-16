use camunda_core::models::DashboardDatePreset;
use gpui_kit::component::accordion::Accordion;
use gpui_kit::component::input::Input;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::OverviewView;
use super::versions::{NUMBER_WIDTH, TRAILING, definition_card, incidents_cell};
use crate::model::{self, ProcessGroup};

impl OverviewView {
    // Панель процессов: фильтр, заголовки колонок и группы с версиями.
    pub(super) fn render_processes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let live = self.preset == DashboardDatePreset::Live;
        let head = h_flex()
            .px_4()
            .py_2()
            .gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Процессы"))
            .child(Tag::secondary().small().rounded_full().child(self.processes.len().to_string()))
            .child(div().flex_1())
            .child(div().flex_1().max_w(px(220.)).child(Input::new(&self.filter).small()));
        let columns = h_flex()
            .pl_3()
            .pr(TRAILING)
            .py_2()
            .gap_3()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(cx.theme().muted_foreground)
            .border_y_1()
            .border_color(cx.theme().border)
            .child(div().flex_1().child("Процесс"))
            .child(div().min_w(NUMBER_WIDTH).text_right().child(if live { "в работе" } else { "стартов" }))
            .child(div().min_w(NUMBER_WIDTH).text_right().child("инциденты"));
        let keys: Vec<SharedString> = self.processes.iter().map(|group| group.key.clone()).collect();
        let accordion = Accordion::new("processes").h_auto().on_toggle_click(cx.listener(move |this, open: &[usize], _, cx| {
            this.expanded_process = open.first().and_then(|ix| keys.get(*ix)).cloned();
            cx.notify();
        }));
        let accordion = self.processes.iter().fold(accordion, |accordion, group| {
            let open = self.expanded_process.as_ref() == Some(&group.key);
            let versions = self.render_versions(group, cx);
            accordion.item(|item| {
                item.open(open)
                    .hover(|style| style.bg(cx.theme().table_hover))
                    .title(process_title(group, cx))
                    .content_style(StyleRefinement::default().p_0())
                    .children(versions)
            })
        });
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .child(head)
            .child(columns)
            .child(div().id("processes").flex_1().min_h_0().overflow_y_scroll().child(accordion))
    }
}

fn process_title(group: &ProcessGroup, cx: &App) -> Div {
    let latest = &group.latest;
    let versions = if group.versions.len() > 1 { format!(" · версий {}", group.versions.len()) } else { String::new() };
    let meta = format!("{} · v{}{versions}{}", latest.key, latest.version, if latest.suspended { " · пауза" } else { "" });
    h_flex()
        .flex_1()
        .min_w_0()
        .gap_3()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(definition_card(group.key.to_string(), latest, div().truncate().child(model::display_name(latest).to_string())))
                .child(
                    div()
                        .truncate()
                        .text_xs()
                        .font_weight(FontWeight::NORMAL)
                        .font_family(crate::platform::theme::mono(cx))
                        .text_color(cx.theme().muted_foreground)
                        .child(meta),
                ),
        )
        .child(div().min_w(NUMBER_WIDTH).text_right().child(model::grouped(group.instances as i64)))
        .child(div().min_w(NUMBER_WIDTH).flex().justify_end().child(incidents_cell(group.incidents, cx)))
}
