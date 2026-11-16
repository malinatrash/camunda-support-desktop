use camunda_core::models::ProcessInstanceDetails;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::*;

use super::InstanceView;
use super::variables::{VariablesChanged, VariablesView};
use crate::bpmn::{BpmnView, Highlights, SelectNode};

impl InstanceView {
    // Показывает свежие данные заявки: подсветка схемы и переменные, при первом показе создаёт панели.
    pub(super) fn show(&mut self, details: ProcessInstanceDetails, window: &mut Window, cx: &mut Context<Self>) {
        let highlights = highlights(&details);
        if let Some(diagram) = &self.diagram {
            diagram.update(cx, |diagram, cx| diagram.update(&details.diagram, highlights, cx));
            if let Some(variables) = &self.variables {
                variables.update(cx, |variables, cx| variables.set_variables(details.variables.clone(), cx));
            }
        } else {
            let variables =
                cx.new(|cx| VariablesView::new(self.connection.clone(), self.instance_id.clone(), details.variables.clone(), window, cx));
            let diagram = cx.new(|_| BpmnView::new(details.diagram.clone(), highlights));
            self._subscriptions = vec![
                cx.subscribe_in(&variables, window, |this, _, event: &VariablesChanged, window, cx| {
                    this.notice = Some(format!("Переменная {} обновлена.", event.0).into());
                    this.reload(window, cx);
                }),
                cx.subscribe_in(&diagram, window, |this, _, event: &SelectNode, window, cx| this.select_node(event.0.clone(), window, cx)),
            ];
            self.variables = Some(variables);
            self.diagram = Some(diagram);
        }
        self.details = Some(details);
        self.error = None;
    }

    pub(super) fn render_diagram(
        &self,
        details: &ProcessInstanceDetails,
        diagram: &Entity<BpmnView>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_resizable("instance-split")
            .child(
                resizable_panel().child(
                    div()
                        .size_full()
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius)
                        .overflow_hidden()
                        .child(diagram.clone()),
                ),
            )
            .child(resizable_panel().size(px(360.)).size_range(px(280.)..px(640.)).child(self.render_side(details, cx)))
    }

    fn select_node(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let teleportable = self.details.as_ref().is_some_and(|details| details.diagram.teleport_targets().any(|node| node.id == id));
        self.selected_node = Some(id.clone());
        if teleportable && self.can_teleport() {
            self.open_teleport(Some(id), window, cx);
        }
        cx.notify();
    }
}

fn highlights(details: &ProcessInstanceDetails) -> Highlights {
    let mut highlights = Highlights::default();
    highlights.active.extend(details.active_activities.iter().map(|activity| activity.activity_id.clone()));
    for incident in &details.incidents {
        if let Some(activity) = incident.failed_activity_id.clone().or_else(|| incident.activity_id.clone()) {
            *highlights.incidents.entry(activity).or_default() += 1;
        }
    }
    highlights
        .completed
        .extend(details.activity_history.iter().filter(|item| item.completed_count > 0).map(|item| item.activity_id.clone()));
    highlights
}
