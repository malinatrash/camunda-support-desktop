mod editor;
mod list;
mod validation;

use camunda_core::models::{CamundaConnection, ProcessVariable};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Sizable, WindowExt as _, h_flex};
use gpui_kit::*;

use super::panel::list_panel;
use editor::{EditorEvent, VariableEditor};

pub struct VariablesChanged(pub String);

pub struct VariablesView {
    connection: CamundaConnection,
    instance_id: String,
    variables: Vec<ProcessVariable>,
    query: Entity<InputState>,
    editor: Option<Entity<VariableEditor>>,
    _subscriptions: Vec<Subscription>,
    _editor_subscription: Option<Subscription>,
}

impl EventEmitter<VariablesChanged> for VariablesView {}

impl VariablesView {
    pub fn new(
        connection: CamundaConnection,
        instance_id: String,
        variables: Vec<ProcessVariable>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Поиск по ключу, типу или значению"));
        let subscription = cx.subscribe(&query, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self { connection, instance_id, variables, query, editor: None, _subscriptions: vec![subscription], _editor_subscription: None }
    }

    pub fn set_variables(&mut self, variables: Vec<ProcessVariable>, cx: &mut Context<Self>) {
        self.variables = variables;
        cx.notify();
    }

    // Открывает редактор переменной в боковой панели.
    fn edit(&mut self, variable: ProcessVariable, window: &mut Window, cx: &mut Context<Self>) {
        let title = format!("Переменная {}", variable.name);
        let (connection, instance_id) = (self.connection.clone(), self.instance_id.clone());
        let editor = cx.new(|cx| VariableEditor::new(connection, instance_id, variable, window, cx));
        self._editor_subscription = Some(cx.subscribe_in(&editor, window, |this, _, event: &EditorEvent, window, cx| {
            this.editor = None;
            window.close_sheet(cx);
            if let EditorEvent::Saved(name) = event {
                cx.emit(VariablesChanged(name.clone()));
            }
        }));
        self.editor = Some(editor.clone());
        window.open_sheet(cx, move |sheet, _, _| sheet.title(title.clone()).size(relative(0.45)).child(editor.clone()));
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
            .flex_1()
            .gap_3()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Переменные Camunda"))
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!("{} из {}", visible.len(), self.variables.len())))
            .child(div().flex_1())
            .child(div().w(relative(0.35)).min_w(px(200.)).child(Input::new(&self.query).small()));
        list_panel(head, cx).child(div().id("variables").flex_1().min_h_0().overflow_y_scroll().child(self.render_list(visible, cx)))
    }
}
