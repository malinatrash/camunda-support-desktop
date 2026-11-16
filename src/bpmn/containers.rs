use camunda_core::models::BpmnDiagram;
use gpui_kit::*;

use super::kind::is_container;
use super::stage::Stage;

// Рамки пулов, дорожек и развёрнутых подпроцессов в видимой области.
pub fn paint_containers(diagram: &BpmnDiagram, stage: &Stage, window: &mut Window) {
    let zoom = stage.zoom;
    for node in diagram.nodes.iter().filter(|node| is_container(node)) {
        let rect = stage.rect(node);
        if rect.intersects(&stage.visible) {
            let (radius, style) = match node.node_type.as_str() {
                "participant" | "lane" => (px(0.), BorderStyle::Solid),
                _ if node.triggered_by_event => (px(10. * zoom), BorderStyle::Dashed),
                _ => (px(10. * zoom), BorderStyle::Solid),
            };
            window.paint_quad(quad(rect, radius, transparent_black(), px((1.2 * zoom).max(1.)), stage.palette.stroke.opacity(0.6), style));
        }
    }
}
