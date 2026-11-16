use camunda_core::models::{ProcessVariableCatalog, ProcessVariableDescriptor, VariableValueType};
use gpui_kit::component::select::{SearchableVec, SelectItem};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use super::SearchView;
use crate::platform::services;

impl SearchView {
    pub(super) fn load_catalog(&mut self, key: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        self.catalog = None;
        self.catalog_error = None;
        self.variable.update(cx, |state, cx| state.set_items(SearchableVec::new(Vec::<VariableItem>::new()), window, cx));
        let Some(key) = key else {
            self.catalog_load = None;
            cx.notify();
            return;
        };
        let connection = self.connection.clone();
        let request = services::request(cx, move |api| async move { api.load_process_variable_catalog(&connection, &key).await });
        self.catalog_load = Some(cx.spawn_in(window, async move |this, cx| {
            let result = request.await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(catalog) => {
                        let items = variable_items(&catalog);
                        this.variable.update(cx, |state, cx| state.set_items(SearchableVec::new(items), window, cx));
                        this.catalog = Some(catalog);
                    }
                    Err(error) => this.catalog_error = Some(error),
                }
                this.catalog_load = None;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    // Сбрасывает кэш переменных процесса и читает их заново.
    pub(super) fn refresh_catalog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.selected_process(cx) else { return };
        cx.global::<services::Services>().api.invalidate_process_variable_catalog(&self.connection, &key);
        self.load_catalog(Some(key.into()), window, cx);
    }
}

// Переменные каталога с подписью на текущем языке.
pub(super) fn variable_items(catalog: &ProcessVariableCatalog) -> Vec<VariableItem> {
    let item = |variable: &ProcessVariableDescriptor| VariableItem {
        name: variable.name.clone().into(),
        subtitle: tr!("search.variable.subtitle", types = variable.types.join(", "), count = variable.occurrences),
    };
    catalog.variables.iter().map(item).collect()
}

#[derive(Clone)]
pub struct VariableItem {
    name: SharedString,
    subtitle: SharedString,
}

impl SelectItem for VariableItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .child(div().font_family(crate::platform::theme::mono(cx)).child(self.name.clone()))
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(self.subtitle.clone()))
    }

    fn value(&self) -> &SharedString {
        &self.name
    }
}

#[derive(Clone)]
pub struct TypeItem {
    label: SharedString,
    value: VariableValueType,
}

impl TypeItem {
    pub(super) fn all() -> Vec<TypeItem> {
        [
            ("search.type.auto", VariableValueType::Auto),
            ("search.type.string", VariableValueType::String),
            ("search.type.number", VariableValueType::Number),
            ("search.type.boolean", VariableValueType::Boolean),
        ]
        .into_iter()
        .map(|(key, value)| TypeItem { label: tr!(key), value })
        .collect()
    }
}

impl SelectItem for TypeItem {
    type Value = VariableValueType;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &VariableValueType {
        &self.value
    }
}
