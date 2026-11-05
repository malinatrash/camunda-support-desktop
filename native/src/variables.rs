use camunda_core::models::{CamundaConnection, ProcessVariable, ProcessVariableUpdate};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::services;
use crate::ui;

const READ_ONLY_TYPES: [&str; 4] = ["object", "file", "bytes", "null"];

pub struct VariablesChanged(pub String);

struct Editing {
    variable: ProcessVariable,
    value: Entity<TextareaState>,
    error: Option<String>,
}

pub struct VariablesView {
    connection: CamundaConnection,
    instance_id: String,
    variables: Vec<ProcessVariable>,
    query: Entity<InputState>,
    editing: Option<Editing>,
    save: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<VariablesChanged> for VariablesView {}

impl VariablesView {
    pub fn new(connection: CamundaConnection, instance_id: String, variables: Vec<ProcessVariable>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Поиск по ключу, типу или значению"));
        let subscription = cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self { connection, instance_id, variables, query, editing: None, save: None, _subscriptions: vec![subscription] }
    }

    pub fn set_variables(&mut self, variables: Vec<ProcessVariable>, cx: &mut Context<Self>) {
        self.variables = variables;
        cx.notify();
    }

    fn edit(&mut self, variable: ProcessVariable, window: &mut Window, cx: &mut Context<Self>) {
        let initial = variable.value.clone();
        let value = cx.new(|cx| TextareaState::new(window, cx).default_value(initial));
        self.editing = Some(Editing { variable, value, error: None });
        cx.notify();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = &mut self.editing else { return };
        let value = editing.value.read(cx).value().to_string();
        if let Some(error) = validate(&editing.variable.value_type, &value) {
            editing.error = Some(error);
            cx.notify();
            return;
        }
        let update = ProcessVariableUpdate {
            name: editing.variable.name.clone(),
            value_type: editing.variable.value_type.clone(),
            value,
            value_info: editing.variable.value_info.clone(),
        };
        let name = update.name.clone();
        let (connection, instance_id) = (self.connection.clone(), self.instance_id.clone());
        let request = services::request(cx, move |api| async move { api.update_process_variable(&connection, &instance_id, &update).await });
        self.save = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| {
                this.save = None;
                match result {
                    Ok(()) => {
                        this.editing = None;
                        cx.emit(VariablesChanged(name));
                    }
                    Err(error) => {
                        if let Some(editing) = &mut this.editing {
                            editing.error = Some(error);
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn render_editor(&self, editing: &Editing, cx: &mut Context<Self>) -> impl IntoElement {
        let variable = &editing.variable;
        div().absolute().inset_0().flex().items_center().justify_center().bg(black().opacity(0.45)).child(
            v_flex()
                .id("variable-editor")
                .w(px(640.))
                .p_5()
                .gap_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().popover)
                .shadow_2xl()
                .occlude()
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" && this.save.is_none() {
                        this.editing = None;
                        cx.notify();
                    }
                }))
                .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(format!("Переменная {}", variable.name)))
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child(hint(&variable.value_type)))
                .child(Textarea::new(&editing.value).h(px(160.)))
                .when_some(editing.error.clone(), |this, error| {
                    this.child(div().px_3().py_2().rounded_md().bg(cx.theme().danger.opacity(0.1)).text_color(cx.theme().danger).child(error))
                })
                .child(
                    h_flex()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("variable-cancel").label("Отмена").ghost().on_click(cx.listener(|this, _, _, cx| {
                            this.editing = None;
                            cx.notify();
                        })))
                        .child(
                            Button::new("variable-save")
                                .label("Сохранить")
                                .primary()
                                .loading(self.save.is_some())
                                .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
                        ),
                ),
        )
    }
}

fn editable(variable: &ProcessVariable) -> bool {
    !READ_ONLY_TYPES.contains(&variable.value_type.to_lowercase().as_str())
}

fn hint(value_type: &str) -> String {
    match value_type.to_lowercase().as_str() {
        "boolean" => "Введите true или false.".into(),
        "short" | "integer" | "long" => "Введите целое число без пробелов и разделителей.".into(),
        "double" => "Введите число, дробная часть отделяется точкой.".into(),
        "date" => "Сохранится тип Date; используйте формат даты Camunda с часовым поясом.".into(),
        "json" => "Введите полный корректный JSON. Он сохранится строковым JSON-значением Camunda.".into(),
        _ => format!("Тип переменной останется {value_type}."),
    }
}

fn validate(value_type: &str, value: &str) -> Option<String> {
    let value = value.trim();
    match value_type.to_lowercase().as_str() {
        "boolean" => (!value.eq_ignore_ascii_case("true") && !value.eq_ignore_ascii_case("false")).then(|| "Допустимы только true или false.".into()),
        "short" => value.parse::<i16>().is_err().then(|| "Значение выходит за диапазон Short или не является целым числом.".into()),
        "integer" => value.parse::<i32>().is_err().then(|| "Значение выходит за диапазон Integer или не является целым числом.".into()),
        "long" => value.parse::<i64>().is_err().then(|| "Значение выходит за диапазон Long или не является целым числом.".into()),
        "double" => (!value.parse::<f64>().is_ok_and(f64::is_finite)).then(|| "Введите конечное число в формате Double.".into()),
        _ => None,
    }
}

impl Render for VariablesView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.query.read(cx).value().trim().to_lowercase();
        let visible: Vec<&ProcessVariable> = self
            .variables
            .iter()
            .filter(|v| query.is_empty() || [&v.name, &v.value_type, &v.value].iter().any(|text| text.to_lowercase().contains(&query)))
            .collect();
        let head = h_flex()
            .w_full()
            .gap_3()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Переменные Camunda"))
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!("{} из {}", visible.len(), self.variables.len())))
            .child(div().flex_1())
            .child(div().w(px(280.)).child(Input::new(&self.query).small()));
        let rows = visible.into_iter().map(|variable| {
            let copy_value = variable.value.clone();
            let target = variable.clone();
            let can_edit = editable(variable);
            h_flex()
                .px_4()
                .py_3()
                .gap_4()
                .items_start()
                .border_b_1()
                .border_color(cx.theme().border)
                .text_sm()
                .child(
                    v_flex()
                        .w(px(260.))
                        .flex_none()
                        .child(div().truncate().font_family(crate::theme::MONO).font_weight(FontWeight::SEMIBOLD).child(variable.name.clone()))
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(variable.value_type.clone())),
                )
                .child(div().flex_1().min_w_0().line_clamp(3).font_family(crate::theme::MONO).text_xs().child(variable.value.clone()))
                .child(
                    h_flex()
                        .flex_none()
                        .gap_1()
                        .child(Button::new(SharedString::from(format!("copy-{}", variable.name))).label("Копировать").ghost().small().on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_value.clone()));
                        }))
                        .child(
                            Button::new(SharedString::from(format!("edit-{}", variable.name)))
                                .label(if can_edit { "Изменить" } else { "Только чтение" })
                                .small()
                                .disabled(!can_edit)
                                .on_click(cx.listener(move |this, _, window, cx| this.edit(target.clone(), window, cx))),
                        ),
                )
        });
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(ui::section(head, cx).flex_1().child(div().id("variables").flex_1().overflow_y_scroll().children(rows)))
            .when_some(self.editing.as_ref(), |this, editing| this.child(deferred(self.render_editor(editing, cx))))
    }
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn validates_numbers_and_booleans() {
        assert!(validate("Integer", "42").is_none());
        assert!(validate("Integer", "4 2").is_some());
        assert!(validate("Boolean", "TRUE").is_none());
        assert!(validate("Short", "40000").is_some());
        assert!(validate("Double", "1.5").is_none());
        assert!(validate("String", "что угодно").is_none());
    }
}
