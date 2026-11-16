use camunda_core::models::{CamundaConnection, ProcessVariable, ProcessVariableUpdate};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::form::{field, v_form};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::validation::{hint, validate};
use crate::platform::services;

pub enum EditorEvent {
    Saved(String),
    Cancel,
}

pub struct VariableEditor {
    connection: CamundaConnection,
    instance_id: String,
    variable: ProcessVariable,
    value: Entity<TextareaState>,
    error: Option<String>,
    save: Option<Task<()>>,
}

impl EventEmitter<EditorEvent> for VariableEditor {}

impl VariableEditor {
    pub fn new(
        connection: CamundaConnection,
        instance_id: String,
        variable: ProcessVariable,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial = variable.value.clone();
        let value = cx.new(|cx| TextareaState::new(window, cx).default_value(initial));
        Self { connection, instance_id, variable, value, error: None, save: None }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let value = self.value.read(cx).value().to_string();
        if let Some(error) = validate(&self.variable.value_type, &value) {
            self.error = Some(error);
            cx.notify();
            return;
        }
        let update = ProcessVariableUpdate {
            name: self.variable.name.clone(),
            value_type: self.variable.value_type.clone(),
            value,
            value_info: self.variable.value_info.clone(),
        };
        let name = update.name.clone();
        let (connection, instance_id) = (self.connection.clone(), self.instance_id.clone());
        let request =
            services::request(cx, move |api| async move { api.update_process_variable(&connection, &instance_id, &update).await });
        self.save = Some(cx.spawn(async move |this, cx| {
            let result = request.await;
            this.update(cx, |this, cx| {
                this.save = None;
                match result {
                    Ok(()) => cx.emit(EditorEvent::Saved(name)),
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }
}

impl Render for VariableEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                v_form().child(
                    field()
                        .label(self.variable.value_type.clone())
                        .description(hint(&self.variable.value_type))
                        .child(Textarea::new(&self.value).h(px(240.))),
                ),
            )
            .when_some(self.error.clone(), |this, error| this.child(Alert::error("variable-error", error)))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("variable-cancel")
                            .label(tr!("instance.variables.cancel"))
                            .ghost()
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(EditorEvent::Cancel))),
                    )
                    .child(
                        Button::new("variable-save")
                            .label(tr!("instance.variables.save"))
                            .primary()
                            .loading(self.save.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
                    ),
            )
    }
}
