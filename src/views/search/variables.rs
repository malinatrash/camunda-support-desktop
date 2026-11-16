use camunda_core::models::VariableValueType;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::Input;
use gpui_kit::component::select::Select;
use gpui_kit::component::{ActiveTheme, Disableable, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::SearchView;
use crate::platform::services;

impl SearchView {
    pub(super) fn render_variable_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let process_selected = self.selected_process(cx).is_some();
        let step: String = if !process_selected {
            "1. Выберите процесс".into()
        } else if self.catalog_load.is_some() {
            "2. Читаем переменные активных заявок…".into()
        } else if let Some(catalog) = &self.catalog {
            format!(
                "2. Найдено ключей: {} в {} активных заявках{} · кэш между сессиями до 24 ч.",
                catalog.variables.len(),
                catalog.inspected_instance_count,
                if catalog.instances_truncated { " (выборка ограничена)" } else { "" }
            )
        } else {
            "2. Выберите переменную".into()
        };
        let process = Select::new(&self.process)
            .placeholder("Процесс")
            .search_placeholder("Название или ключ процесса")
            .menu_width(px(420.))
            .accessibility_label("Процесс");
        let variable = Select::new(&self.variable)
            .placeholder("Переменная")
            .search_placeholder("Имя переменной")
            .menu_width(px(380.))
            .accessibility_label("Переменная")
            .disabled(!process_selected || self.catalog_load.is_some());
        let refresh = Button::new("refresh-catalog")
            .icon(IconName::RefreshCw)
            .ghost()
            .tooltip("Перечитать переменные процесса")
            .loading(self.catalog_load.is_some())
            .disabled(!process_selected)
            .on_click(cx.listener(|this, _, window, cx| this.refresh_catalog(window, cx)));
        let find = Button::new("search-variable")
            .label("Найти")
            .primary()
            .loading(self.search.is_some())
            .on_click(cx.listener(|this, _, _, cx| this.search_by_variable(cx)));
        v_flex()
            .gap_4()
            .child(
                v_form()
                    .columns(4)
                    .child(field().label("Процесс").child(process))
                    .child(
                        field().label("Переменная").child(h_flex().gap_1().child(div().flex_1().min_w_0().child(variable)).child(refresh)),
                    )
                    .child(field().label("Значение").child(Input::new(&self.value)))
                    .child(field().label("Тип значения").child(Select::new(&self.value_type).accessibility_label("Тип значения")))
                    .footer(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .child(div().flex_1().text_sm().text_color(cx.theme().muted_foreground).child(step))
                            .child(find),
                    ),
            )
            .when_some(self.definitions_error.clone().or_else(|| self.catalog_error.clone()), |this, error| {
                this.child(Alert::error("catalog-error", error))
            })
    }

    pub(super) fn search_by_variable(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.selected_process(cx) else {
            self.error = Some("Сначала выберите процесс.".into());
            cx.notify();
            return;
        };
        let Some(name) = self.variable.read(cx).selected_value().map(|name| name.to_string()) else {
            self.error = Some("Выберите переменную из списка процесса.".into());
            cx.notify();
            return;
        };
        let value = self.value.read(cx).value().to_string();
        if value.trim().is_empty() {
            self.error = Some("Укажите точное значение переменной.".into());
            cx.notify();
            return;
        }
        let value_type = self.value_type.read(cx).selected_value().copied().unwrap_or(VariableValueType::Auto);
        let connection = self.connection.clone();
        let request =
            services::request(
                cx,
                move |api| async move { api.search_process_instances(&connection, &key, &name, &value, value_type).await },
            );
        self.start(request, cx);
    }
}
