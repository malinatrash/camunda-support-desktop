mod dialog;
mod form;
mod targets;

use camunda_core::models::{ActiveActivityInstance, BpmnNode, CamundaConnection, ProcessInstanceDetails, TeleportRequest};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

use crate::platform::services;

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
        let subscriptions =
            vec![cx.subscribe(&source_query, notify), cx.subscribe(&target_query, notify), cx.subscribe(&annotation, notify)];
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
        let call =
            services::request(cx, move |api| async move { api.teleport_process_instance(&connection, &instance_id, &request).await });
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
}
