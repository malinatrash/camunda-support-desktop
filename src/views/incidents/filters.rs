use std::collections::HashSet;

use camunda_core::models::{DashboardDatePreset, ProcessIncident};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::select::Select;
use gpui_kit::component::tab::TabBar;
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::{IconName, Sizable, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::IncidentsView;
use super::groups::group_key;
use crate::model;

impl IncidentsView {
    pub(super) fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let presets =
            [(DashboardDatePreset::Live, "Сейчас"), (DashboardDatePreset::Today, "Сегодня"), (DashboardDatePreset::Yesterday, "Вчера")];
        h_flex().gap_4().child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).child("Инциденты")).child(
            Toolbar::new("incidents-toolbar")
                .small()
                .children(presets.into_iter().map(|(preset, label)| {
                    Button::new(label)
                        .label(label)
                        .map(|button| if self.preset == preset { button.secondary() } else { button.ghost() })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.preset = preset;
                            this.reload(window, cx);
                        }))
                }))
                .child(
                    Button::new("reload")
                        .icon(IconName::RefreshCw)
                        .ghost()
                        .tooltip("Обновить")
                        .loading(self.load.is_some())
                        .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
                ),
        )
    }

    pub(super) fn render_filters(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut types: Vec<(String, usize)> = Vec::new();
        for incident in &self.incidents {
            match types.iter_mut().find(|(kind, _)| *kind == incident.incident_type) {
                Some((_, count)) => *count += 1,
                None => types.push((incident.incident_type.clone(), 1)),
            }
        }
        types.sort_by_key(|item| std::cmp::Reverse(item.1));
        types.truncate(6);
        let kinds: Vec<String> = types.iter().map(|(kind, _)| kind.clone()).collect();
        let selected = self.selected_type.as_ref().and_then(|kind| kinds.iter().position(|known| known == kind)).map_or(0, |ix| ix + 1);
        let labels = types.iter().map(|(kind, count)| format!("{} {count}", model::incident_type_label(kind)));
        h_flex()
            .gap_2()
            .flex_wrap()
            .child(div().flex_1().min_w(px(220.)).child(Select::new(&self.definition).small().accessibility_label("Процесс")))
            .child(div().flex_1().min_w(px(220.)).child(Input::new(&self.query).small()))
            .child(TabBar::new("incident-types").segmented().small().selected_index(selected).child("Все типы").children(labels).on_click(
                cx.listener(move |this, index: &usize, _, cx| {
                    this.selected_type = index.checked_sub(1).and_then(|ix| kinds.get(ix).cloned());
                    this.sync_table(cx);
                }),
            ))
    }

    pub(super) fn filtered(&self, cx: &App) -> Vec<ProcessIncident> {
        let query = self.query.read(cx).value().trim().to_lowercase();
        let matches = |incident: &&ProcessIncident| {
            query.is_empty()
                || [
                    Some(&incident.id),
                    incident.message.as_ref(),
                    incident.activity_id.as_ref(),
                    incident.failed_activity_id.as_ref(),
                    incident.process_instance_id.as_ref(),
                    incident.process_definition_id.as_ref(),
                    incident.configuration.as_ref(),
                ]
                .into_iter()
                .flatten()
                .any(|value| value.to_lowercase().contains(&query))
        };
        let of_type = |incident: &&ProcessIncident| self.selected_type.as_ref().is_none_or(|kind| incident.incident_type == *kind);
        self.incidents.iter().filter(of_type).filter(matches).cloned().collect()
    }

    // Кладёт в таблицу отфильтрованные инциденты выбранной группы, сохраняя видимые отметки.
    pub(super) fn sync_table(&mut self, cx: &mut Context<Self>) {
        let filtered = self.filtered(cx);
        if self.selected_group.as_ref().is_some_and(|key| !filtered.iter().any(|incident| group_key(incident) == key.as_ref())) {
            self.selected_group = None;
        }
        let in_group = |incident: &ProcessIncident| self.selected_group.as_ref().is_none_or(|key| group_key(incident) == key.as_ref());
        let rows: Vec<ProcessIncident> = filtered.into_iter().filter(in_group).collect();
        self.table.update(cx, |table, cx| {
            let delegate = table.delegate_mut();
            let visible: HashSet<&str> = rows.iter().map(|incident| incident.id.as_str()).collect();
            delegate.selected.retain(|id| visible.contains(id.as_str()));
            delegate.rows = rows;
            table.refresh(cx);
        });
        cx.notify();
    }
}
