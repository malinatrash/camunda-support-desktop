use camunda_core::models::{ActiveActivityInstance, BpmnNode, CamundaConnection, Environment, ProcessInstanceDetails, TeleportRequest};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Disableable, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::services;

pub enum TeleportEvent {
    Done,
    Cancel,
}

pub struct TeleportView {
    connection: CamundaConnection,
    instance_id: String,
    sources: Vec<ActiveActivityInstance>,
    targets: Vec<BpmnNode>,
    source: Option<String>,
    target: Option<String>,
    source_query: Entity<InputState>,
    target_query: Entity<InputState>,
    annotation: Entity<InputState>,
    confirmed: bool,
    error: Option<String>,
    run: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<TeleportEvent> for TeleportView {}

impl TeleportView {
    pub fn new(
        connection: CamundaConnection,
        details: &ProcessInstanceDetails,
        target: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let sources: Vec<_> = details.active_activities.iter().filter(|activity| activity.cancellable).cloned().collect();
        let mut targets: Vec<_> = details.diagram.teleport_targets().cloned().collect();
        if let Some(index) = target.as_ref().and_then(|id| targets.iter().position(|node| &node.id == id)) {
            let chosen = targets.remove(index);
            targets.insert(0, chosen);
        }
        let source_query = cx.new(|cx| InputState::new(window, cx).placeholder("Поиск по названию или ID"));
        let target_query = cx.new(|cx| InputState::new(window, cx).placeholder("Название, ID или topic кубика"));
        let annotation = cx.new(|cx| InputState::new(window, cx).placeholder("Например: WPL-800, повтор отправки в HAOS"));
        let notify = |_: &mut Self, _: Entity<InputState>, event: &InputEvent, cx: &mut Context<Self>| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        };
        let subscriptions = vec![cx.subscribe(&source_query, notify), cx.subscribe(&target_query, notify), cx.subscribe(&annotation, notify)];
        Self {
            connection,
            instance_id: details.instance.id.clone(),
            source: sources.first().map(|activity| activity.id.clone()),
            target: target.filter(|id| targets.iter().any(|node| &node.id == id)),
            sources,
            targets,
            source_query,
            target_query,
            annotation,
            confirmed: false,
            error: None,
            run: None,
            _subscriptions: subscriptions,
        }
    }

    fn production(&self) -> bool {
        self.connection.environment == Environment::Production
    }

    fn can_execute(&self, cx: &App) -> bool {
        self.run.is_none()
            && self.source.is_some()
            && self.target.is_some()
            && !self.annotation.read(cx).value().trim().is_empty()
            && (!self.production() || self.confirmed)
    }

    fn execute(&mut self, cx: &mut Context<Self>) {
        let (Some(source), Some(target)) = (self.source.clone(), self.target.clone()) else { return };
        let request = TeleportRequest {
            source_activity_instance_id: source,
            target_activity_id: target,
            annotation: self.annotation.read(cx).value().trim().to_string(),
            skip_custom_listeners: true,
            skip_io_mappings: true,
        };
        let (connection, instance_id) = (self.connection.clone(), self.instance_id.clone());
        let call = services::request(cx, move |api| async move { api.teleport_process_instance(&connection, &instance_id, &request).await });
        self.error = None;
        self.run = Some(cx.spawn(async move |this, cx| {
            let result = call.await;
            this.update(cx, |this, cx| {
                this.run = None;
                match result {
                    Ok(()) => cx.emit(TeleportEvent::Done),
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn picker(&self, step: &'static str, title: &'static str, query: &Entity<InputState>, rows: Vec<AnyElement>, cx: &App) -> impl IntoElement {
        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .gap_2()
            .p_4()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(cx.theme().link).child(step))
            .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
            .child(Input::new(query).small())
            .child(v_flex().id(title).flex_1().min_h_0().gap_1().overflow_y_scroll().children(rows))
    }
}

fn matches(query: &str, values: &[Option<&str>]) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || values.iter().flatten().any(|value| value.to_lowercase().contains(&query))
}

fn option_row(id: SharedString, selected: bool, title: String, meta: String, cx: &App) -> Stateful<Div> {
    v_flex()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if selected { cx.theme().link } else { transparent_black() })
        .when(selected, |this| this.bg(cx.theme().link.opacity(0.1)))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().table_hover))
        .child(div().truncate().font_weight(FontWeight::MEDIUM).child(title))
        .child(div().truncate().text_xs().text_color(cx.theme().muted_foreground).child(meta))
}

impl Render for TeleportView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let source_query = self.source_query.read(cx).value().to_string();
        let target_query = self.target_query.read(cx).value().to_string();
        let sources: Vec<AnyElement> = self
            .sources
            .iter()
            .filter(|a| matches(&source_query, &[a.activity_name.as_deref(), Some(&a.activity_id), Some(&a.activity_type), Some(&a.id)]))
            .map(|activity| {
                let id = activity.id.clone();
                option_row(
                    SharedString::from(format!("source-{}", activity.id)),
                    self.source.as_deref() == Some(activity.id.as_str()),
                    activity.activity_name.clone().unwrap_or_else(|| activity.activity_id.clone()),
                    format!("{} · {}", activity.activity_type, activity.activity_id),
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.source = Some(id.clone());
                    cx.notify();
                }))
                .into_any_element()
            })
            .collect();
        let targets: Vec<AnyElement> = self
            .targets
            .iter()
            .filter(|n| matches(&target_query, &[n.name.as_deref(), Some(&n.id), Some(&n.node_type), n.topic.as_deref()]))
            .map(|node| {
                let id = node.id.clone();
                option_row(
                    SharedString::from(format!("target-{}", node.id)),
                    self.target.as_deref() == Some(node.id.as_str()),
                    node.name.clone().unwrap_or_else(|| node.id.clone()),
                    format!("{} · {}{}", node.node_type, node.id, node.topic.as_ref().map(|topic| format!(" · {topic}")).unwrap_or_default()),
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.target = Some(id.clone());
                    cx.notify();
                }))
                .into_any_element()
            })
            .collect();
        let can_execute = self.can_execute(cx);
        let production = self.production();

        v_flex()
            .id("teleport")
            .w(px(940.))
            .h(px(720.))
            .max_h_full()
            .p_5()
            .gap_4()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .shadow_2xl()
            .overflow_hidden()
            .occlude()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && this.run.is_none() {
                    cx.emit(TeleportEvent::Cancel);
                }
            }))
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Телепорт заявки"))
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Выберите активный экземпляр и кубик, перед которым продолжится выполнение.")),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .gap_4()
                    .child(self.picker("1 · ОТКУДА", "Активный элемент", &self.source_query, sources, cx))
                    .child(self.picker("2 · КУДА", "Целевой кубик BPMN", &self.target_query, targets, cx)),
            )
            .child(
                v_flex()
                    .gap_1p5()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Комментарий для истории Camunda"))
                    .child(Input::new(&self.annotation)),
            )
            .when(production, |this| {
                this.child(
                    Checkbox::new("confirm-production")
                        .label("Понимаю, что это прод, и телепортирую заявку осознанно")
                        .checked(self.confirmed)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.confirmed = *checked;
                            cx.notify();
                        })),
                )
            })
            .when_some(self.error.clone(), |this, error| {
                this.child(div().px_3().py_2().rounded_md().bg(cx.theme().danger.opacity(0.1)).text_color(cx.theme().danger).child(error))
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("cancel").label("Отмена").ghost().on_click(cx.listener(|_, _, _, cx| cx.emit(TeleportEvent::Cancel))))
                    .child(
                        Button::new("execute")
                            .label("Телепортировать")
                            .map(|button| if production { button.danger() } else { button.primary() })
                            .loading(self.run.is_some())
                            .disabled(!can_execute)
                            .on_click(cx.listener(|this, _, _, cx| this.execute(cx))),
                    ),
            )
    }
}
