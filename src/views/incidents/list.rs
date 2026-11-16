use std::collections::HashSet;

use camunda_core::models::ProcessIncident;
use gpui_kit::component::button::Button;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::table::DataTable;
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::*;

use super::IncidentsView;
use super::retry::retryable;

impl IncidentsView {
    pub(super) fn render_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let delegate = self.table.read(cx).delegate();
        let all = !delegate.rows.is_empty() && delegate.selected.len() == delegate.rows.len();
        let retry: Vec<ProcessIncident> =
            delegate.rows.iter().filter(|incident| delegate.selected.contains(&incident.id) && retryable(incident)).cloned().collect();
        let pick_all =
            Checkbox::new("pick-all").label(tr!("incidents.pick_all")).checked(all).on_click(cx.listener(|this, checked: &bool, _, cx| {
                let checked = *checked;
                this.table.update(cx, |table, cx| {
                    let delegate = table.delegate_mut();
                    delegate.selected =
                        if checked { delegate.rows.iter().map(|incident| incident.id.clone()).collect() } else { HashSet::new() };
                    table.refresh(cx);
                });
                cx.notify();
            }));
        let retry_selected = Button::new("retry-selected")
            .label(tr!("incidents.retry_selected", count = retry.len()))
            .outline()
            .small()
            .disabled(retry.is_empty())
            .on_click(cx.listener(move |this, _, window, cx| this.ask_retry(retry.clone(), window, cx)));
        v_flex()
            .size_full()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .px_4()
                    .py_2()
                    .gap_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(pick_all)
                    .child(div().flex_1().text_sm().text_color(cx.theme().muted_foreground).child(tr!("incidents.double_click")))
                    .child(retry_selected),
            )
            .child(div().flex_1().min_h_0().child(DataTable::new(&self.table).stripe(false).bordered(false)))
    }
}
