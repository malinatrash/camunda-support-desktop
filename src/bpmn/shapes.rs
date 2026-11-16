use camunda_core::models::BpmnNode;
use gpui_kit::*;

use super::kind::{is_event, is_gateway};
use super::markers;
use super::palette::Palette;

// Тело узла по типу: круг события, ромб шлюза или прямоугольник задачи, со значком внутри.
pub fn paint_shape(node: &BpmnNode, rect: Bounds<Pixels>, accent: Option<Hsla>, palette: &Palette, zoom: f32, window: &mut Window) {
    let border = accent.unwrap_or(palette.stroke);
    let background = accent.map(|color| color.opacity(0.12)).unwrap_or(palette.background);
    let width = px(if accent.is_some() { 2.4 } else { 1.4 } * zoom.max(0.6));
    let ink = markers::Ink { color: border, background: palette.background, zoom };
    if is_event(node) {
        let border_width = if node.node_type == "endEvent" { width * 2. } else { width };
        let style = if node.interrupting { BorderStyle::Solid } else { BorderStyle::Dashed };
        window.paint_quad(quad(rect, rect.size.width / 2., background, border_width, border, style));
        markers::paint_event(node, rect, &ink, window);
    } else if is_gateway(node) {
        let c = rect.center();
        let diamond = [point(c.x, rect.top()), point(rect.right(), c.y), point(c.x, rect.bottom()), point(rect.left(), c.y)];
        let mut body = PathBuilder::fill();
        body.add_polygon(&diamond, true);
        if let Ok(path) = body.build() {
            window.paint_path(path, background);
        }
        let mut outline = PathBuilder::stroke(width);
        outline.add_polygon(&diamond, true);
        if let Ok(path) = outline.build() {
            window.paint_path(path, border);
        }
        markers::paint_gateway(node, rect, &ink, window);
    } else {
        let width = if node.node_type == "callActivity" { width * 2.5 } else { width };
        let style = if node.triggered_by_event { BorderStyle::Dashed } else { BorderStyle::Solid };
        window.paint_quad(quad(rect, px(8. * zoom), background, width, border, style));
        markers::paint_task(node, rect, &ink, window);
    }
}
