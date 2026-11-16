use camunda_core::models::ProcessDefinitionSummary;
use gpui_kit::component::select::{SearchableVec, SelectItem};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use super::SearchView;
use crate::model;
use crate::platform::services;

impl SearchView {
    pub(super) fn load_definitions(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Task<()> {
        let connection = self.connection.clone();
        let request = services::request(cx, move |api| async move { api.load_search_process_definitions(&connection).await });
        cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(definitions) => {
                        let items: Vec<ProcessItem> = definitions.iter().map(ProcessItem::new).collect();
                        this.process.update(cx, |state, cx| state.set_items(SearchableVec::new(items), window, cx));
                    }
                    Err(error) => this.definitions_error = Some(error),
                }
                cx.notify();
            })
            .ok();
        })
    }

    pub(super) fn selected_process(&self, cx: &App) -> Option<String> {
        self.process.read(cx).selected_value().map(|key| key.to_string())
    }
}

#[derive(Clone)]
pub struct ProcessItem {
    key: SharedString,
    title: SharedString,
    subtitle: SharedString,
}

impl ProcessItem {
    fn new(definition: &ProcessDefinitionSummary) -> Self {
        Self {
            key: definition.key.clone().into(),
            title: model::display_name(definition).to_string().into(),
            subtitle: format!("{} · v{}", definition.key, definition.version).into(),
        }
    }
}

impl SelectItem for ProcessItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex().child(self.title.clone()).child(
            div()
                .text_xs()
                .font_family(crate::platform::theme::mono(cx))
                .text_color(cx.theme().muted_foreground)
                .child(self.subtitle.clone()),
        )
    }

    fn value(&self) -> &SharedString {
        &self.key
    }

    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.title.to_lowercase().contains(&query) || self.key.to_lowercase().contains(&query)
    }
}
