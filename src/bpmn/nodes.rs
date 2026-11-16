use camunda_core::models::BpmnDiagram;
use gpui_kit::*;

use super::Highlights;
use super::kind::{is_container, is_event, is_gateway};
use super::labels::{LABEL_FONT, LabelCache, label_lines, prepare_labels};
use super::shapes::paint_shape;
use super::stage::Stage;
use super::text::{paint_label, paint_text};

// Видимые узлы с подсветкой статуса, выделением, подписью и счётчиком инцидентов.
pub fn paint_nodes(
    diagram: &BpmnDiagram,
    highlights: &Highlights,
    labels: &LabelCache,
    selected: Option<&str>,
    stage: &Stage,
    window: &mut Window,
    cx: &mut App,
) {
    let (zoom, palette) = (stage.zoom, stage.palette);
    // Размер шрифта с шагом 1 px: при плавном зуме глифы берутся из кеша, а не растеризуются каждый кадр.
    let font_size = px((LABEL_FONT * zoom).round());
    prepare_labels(diagram, labels, window);
    for node in diagram.nodes.iter().filter(|node| !is_container(node)) {
        let rect = stage.rect(node);
        if !rect.intersects(&stage.visible) {
            continue;
        }
        let incidents = highlights.incidents.get(&node.id).copied().unwrap_or_default();
        let active = highlights.active.contains(&node.id);
        let completed = highlights.completed.contains(&node.id);
        let accent = if incidents > 0 {
            Some(palette.danger)
        } else if active {
            Some(palette.active)
        } else if completed {
            Some(palette.success)
        } else {
            None
        };
        paint_shape(node, rect, accent, palette, zoom, window);
        if selected == Some(node.id.as_str()) {
            let ring = rect.dilate(px(4.));
            window.paint_quad(quad(ring, px(10. * zoom), transparent_black(), px(2.), palette.active, BorderStyle::Dashed));
        }
        if font_size >= px(5.) {
            let lines = label_lines(node, labels, window);
            paint_label(&lines, rect, !is_event(node) && !is_gateway(node), font_size, palette.text, window, cx);
        }
        if incidents > 0 {
            let badge = Bounds::centered_at(point(rect.right(), rect.top()), size(px(18.), px(18.)));
            window.paint_quad(quad(badge, px(9.), palette.danger, px(0.), transparent_black(), BorderStyle::Solid));
            paint_text(&incidents.to_string().into(), badge, px(11.), white(), window, cx);
        }
    }
}
