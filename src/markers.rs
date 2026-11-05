use std::f32::consts::PI;

use camunda_core::models::{BpmnNode, LoopMarker};
use gpui_kit::*;

// Значки BPMN в духе bpmn-js: определения событий, типы шлюзов, типы задач и маркеры циклов.

pub struct Ink {
    pub color: Hsla,
    pub background: Hsla,
    pub zoom: f32,
}

struct Frame {
    center: Point<Pixels>,
    unit: f32,
}

impl Frame {
    fn at(&self, x: f32, y: f32) -> Point<Pixels> {
        point(self.center.x + px(x * self.unit), self.center.y + px(y * self.unit))
    }

    fn polygon(&self, points: &[(f32, f32)]) -> Vec<Point<Pixels>> {
        points.iter().map(|&(x, y)| self.at(x, y)).collect()
    }
}

pub fn is_throw(node: &BpmnNode) -> bool {
    matches!(node.node_type.as_str(), "endEvent" | "intermediateThrowEvent")
}

pub fn has_double_ring(node: &BpmnNode) -> bool {
    matches!(node.node_type.as_str(), "intermediateCatchEvent" | "intermediateThrowEvent" | "boundaryEvent")
}

pub fn paint_event(node: &BpmnNode, rect: Bounds<Pixels>, ink: &Ink, window: &mut Window) {
    let radius = f32::from(rect.size.width) / 2.;
    if has_double_ring(node) {
        let inner = rect.dilate(px(-3. * ink.zoom));
        let style = if node.interrupting { BorderStyle::Solid } else { BorderStyle::Dashed };
        window.paint_quad(quad(inner, inner.size.width / 2., transparent_black(), px(1.2 * ink.zoom.max(0.6)), ink.color, style));
    }
    let Some(definition) = node.event_definition.as_deref() else { return };
    let frame = Frame { center: rect.center(), unit: radius * 0.5 };
    let filled = is_throw(node);
    match definition {
        "timer" => {
            let face = Bounds::centered_at(frame.center, size(px(frame.unit * 2.), px(frame.unit * 2.)));
            window.paint_quad(quad(face, face.size.width / 2., ink.background, px(1.4 * ink.zoom.max(0.6)), ink.color, BorderStyle::Solid));
            for hour in 0..12 {
                let angle = hour as f32 * PI / 6.;
                let (sin, cos) = angle.sin_cos();
                line(&[frame.at(sin * 0.78, -cos * 0.78), frame.at(sin * 0.95, -cos * 0.95)], 0.9, ink, window);
            }
            line(&[frame.at(0., 0.), frame.at(0.05, -0.62)], 1.4, ink, window);
            line(&[frame.at(0., 0.), frame.at(0.45, 0.12)], 1.4, ink, window);
        }
        "message" => {
            let envelope = frame.polygon(&[(-0.75, -0.5), (0.75, -0.5), (0.75, 0.5), (-0.75, 0.5)]);
            if filled {
                fill(&envelope, ink.color, window);
                line(&frame.polygon(&[(-0.75, -0.5), (0., 0.05), (0.75, -0.5)]), 1.2, &Ink { color: ink.background, ..*ink }, window);
            } else {
                closed(&envelope, ink, window);
                line(&frame.polygon(&[(-0.75, -0.5), (0., 0.05), (0.75, -0.5)]), 1.2, ink, window);
            }
        }
        "error" => shape(
            &frame.polygon(&[(-0.6, 0.65), (-0.25, -0.55), (0.1, 0.15), (0.6, -0.65), (0.25, 0.55), (-0.1, -0.15)]),
            filled,
            ink,
            window,
        ),
        "signal" => shape(&frame.polygon(&[(0., -0.7), (0.68, 0.5), (-0.68, 0.5)]), filled, ink, window),
        "escalation" => shape(&frame.polygon(&[(0., -0.75), (0.5, 0.65), (0., 0.2), (-0.5, 0.65)]), filled, ink, window),
        "compensate" => {
            shape(&frame.polygon(&[(-0.75, 0.), (-0.05, -0.5), (-0.05, 0.5)]), filled, ink, window);
            shape(&frame.polygon(&[(0., 0.), (0.7, -0.5), (0.7, 0.5)]), filled, ink, window);
        }
        "conditional" => {
            closed(&frame.polygon(&[(-0.55, -0.7), (0.55, -0.7), (0.55, 0.7), (-0.55, 0.7)]), ink, window);
            for y in [-0.4, -0.13, 0.13, 0.4] {
                line(&[frame.at(-0.35, y), frame.at(0.35, y)], 1., ink, window);
            }
        }
        "link" => shape(
            &frame.polygon(&[(-0.65, -0.22), (0.1, -0.22), (0.1, -0.52), (0.68, 0.), (0.1, 0.52), (0.1, 0.22), (-0.65, 0.22)]),
            filled,
            ink,
            window,
        ),
        "cancel" => shape(
            &frame.polygon(&[
                (-0.62, -0.38),
                (-0.38, -0.62),
                (0., -0.24),
                (0.38, -0.62),
                (0.62, -0.38),
                (0.24, 0.),
                (0.62, 0.38),
                (0.38, 0.62),
                (0., 0.24),
                (-0.38, 0.62),
                (-0.62, 0.38),
                (-0.24, 0.),
            ]),
            filled,
            ink,
            window,
        ),
        "terminate" => {
            let dot = Bounds::centered_at(frame.center, size(px(frame.unit * 1.7), px(frame.unit * 1.7)));
            window.paint_quad(quad(dot, dot.size.width / 2., ink.color, px(0.), transparent_black(), BorderStyle::Solid));
        }
        _ => {}
    }
}

pub fn paint_gateway(node: &BpmnNode, rect: Bounds<Pixels>, ink: &Ink, window: &mut Window) {
    let frame = Frame { center: rect.center(), unit: f32::from(rect.size.width) / 2. };
    let cross = |frame: &Frame, window: &mut Window| {
        line(&[frame.at(0., -0.42), frame.at(0., 0.42)], 3., ink, window);
        line(&[frame.at(-0.42, 0.), frame.at(0.42, 0.)], 3., ink, window);
    };
    let diagonal = |frame: &Frame, window: &mut Window| {
        line(&[frame.at(-0.3, -0.3), frame.at(0.3, 0.3)], 3., ink, window);
        line(&[frame.at(0.3, -0.3), frame.at(-0.3, 0.3)], 3., ink, window);
    };
    match node.node_type.as_str() {
        "exclusiveGateway" => diagonal(&frame, window),
        "parallelGateway" => cross(&frame, window),
        "complexGateway" => {
            cross(&frame, window);
            diagonal(&frame, window);
        }
        "inclusiveGateway" => {
            let ring = Bounds::centered_at(frame.center, size(px(frame.unit * 0.95), px(frame.unit * 0.95)));
            window.paint_quad(quad(
                ring,
                ring.size.width / 2.,
                transparent_black(),
                px(2.5 * ink.zoom.max(0.5)),
                ink.color,
                BorderStyle::Solid,
            ));
        }
        "eventBasedGateway" => {
            for scale in [1.05, 0.88] {
                let ring = Bounds::centered_at(frame.center, size(px(frame.unit * scale), px(frame.unit * scale)));
                window.paint_quad(quad(
                    ring,
                    ring.size.width / 2.,
                    transparent_black(),
                    px(ink.zoom.max(0.6)),
                    ink.color,
                    BorderStyle::Solid,
                ));
            }
            let pentagon: Vec<(f32, f32)> =
                (0..5).map(|index| (index as f32 * 2. * PI / 5. - PI / 2.).sin_cos()).map(|(sin, cos)| (cos * 0.26, sin * 0.26)).collect();
            closed(&frame.polygon(&pentagon), ink, window);
        }
        _ => {}
    }
}

pub fn paint_task(node: &BpmnNode, rect: Bounds<Pixels>, ink: &Ink, window: &mut Window) {
    if ink.zoom >= 0.3 {
        let frame = Frame { center: rect.origin + point(px(15. * ink.zoom), px(15. * ink.zoom)), unit: 8. * ink.zoom };
        paint_task_icon(node, &frame, ink, window);
    }
    let mut markers: Vec<fn(&Frame, &Ink, &mut Window)> = Vec::new();
    if let Some(marker) = node.loop_marker {
        markers.push(match marker {
            LoopMarker::Parallel => parallel_marker,
            LoopMarker::Sequential => sequential_marker,
            LoopMarker::Standard => loop_marker,
        });
    }
    if matches!(node.node_type.as_str(), "subProcess" | "callActivity" | "transaction") && !node.expanded {
        markers.push(collapsed_marker);
    }
    let unit = 6. * ink.zoom;
    let span = markers.len() as f32 * unit * 2.6;
    for (index, marker) in markers.into_iter().enumerate() {
        let x = f32::from(rect.center().x) - span / 2. + unit * 1.3 + index as f32 * unit * 2.6;
        let frame = Frame { center: point(px(x), rect.bottom() - px(unit * 1.6)), unit };
        marker(&frame, ink, window);
    }
}

fn paint_task_icon(node: &BpmnNode, frame: &Frame, ink: &Ink, window: &mut Window) {
    match node.node_type.as_str() {
        "serviceTask" => {
            let teeth: Vec<(f32, f32)> = (0..32)
                .map(|index| {
                    let angle = index as f32 * PI / 16.;
                    let radius = if (index / 2) % 2 == 0 { 1. } else { 0.72 };
                    (angle.cos() * radius, angle.sin() * radius)
                })
                .collect();
            shape(&frame.polygon(&teeth), false, ink, window);
            let hub = Bounds::centered_at(frame.center, size(px(frame.unit * 0.7), px(frame.unit * 0.7)));
            window.paint_quad(quad(hub, hub.size.width / 2., ink.background, px(ink.zoom.max(0.6)), ink.color, BorderStyle::Solid));
        }
        "userTask" => {
            let head = Bounds::centered_at(frame.at(0., -0.45), size(px(frame.unit * 0.8), px(frame.unit * 0.8)));
            window.paint_quad(quad(head, head.size.width / 2., ink.background, px(ink.zoom.max(0.6)), ink.color, BorderStyle::Solid));
            let mut body = PathBuilder::stroke(px(ink.zoom.max(0.6)));
            body.move_to(frame.at(-0.85, 0.95));
            body.curve_to(frame.at(0.85, 0.95), frame.at(0., -0.45));
            body.close();
            if let Ok(path) = body.build() {
                window.paint_path(path, ink.color);
            }
        }
        "scriptTask" => {
            closed(&frame.polygon(&[(-0.6, -0.85), (0.7, -0.85), (0.6, 0.85), (-0.7, 0.85)]), ink, window);
            for y in [-0.45, 0., 0.45] {
                line(&[frame.at(-0.35, y), frame.at(0.35, y)], 1., ink, window);
            }
        }
        "sendTask" | "receiveTask" => {
            let envelope = frame.polygon(&[(-0.9, -0.6), (0.9, -0.6), (0.9, 0.6), (-0.9, 0.6)]);
            let flap = frame.polygon(&[(-0.9, -0.6), (0., 0.05), (0.9, -0.6)]);
            if node.node_type == "sendTask" {
                fill(&envelope, ink.color, window);
                line(&flap, 1., &Ink { color: ink.background, ..*ink }, window);
            } else {
                closed(&envelope, ink, window);
                line(&flap, 1., ink, window);
            }
        }
        "manualTask" => {
            closed(&frame.polygon(&[(-0.9, -0.1), (-0.2, -0.75), (0.85, -0.75), (0.85, 0.75), (-0.9, 0.75)]), ink, window);
            for y in [-0.35, 0.05, 0.4] {
                line(&[frame.at(0.05, y), frame.at(0.85, y)], 1., ink, window);
            }
        }
        "businessRuleTask" => {
            closed(&frame.polygon(&[(-0.9, -0.7), (0.9, -0.7), (0.9, 0.7), (-0.9, 0.7)]), ink, window);
            fill(&frame.polygon(&[(-0.9, -0.7), (0.9, -0.7), (0.9, -0.3), (-0.9, -0.3)]), ink.color, window);
            line(&[frame.at(-0.9, 0.2), frame.at(0.9, 0.2)], 1., ink, window);
            line(&[frame.at(-0.35, -0.3), frame.at(-0.35, 0.7)], 1., ink, window);
        }
        _ => {}
    }
}

fn parallel_marker(frame: &Frame, ink: &Ink, window: &mut Window) {
    for x in [-0.7, 0., 0.7] {
        line(&[frame.at(x, -1.), frame.at(x, 1.)], 1.6, ink, window);
    }
}

fn sequential_marker(frame: &Frame, ink: &Ink, window: &mut Window) {
    for y in [-0.7, 0., 0.7] {
        line(&[frame.at(-1., y), frame.at(1., y)], 1.6, ink, window);
    }
}

fn loop_marker(frame: &Frame, ink: &Ink, window: &mut Window) {
    let mut arc = PathBuilder::stroke(px(1.4 * ink.zoom.max(0.6)));
    arc.move_to(frame.at(-0.4, 0.85));
    arc.arc_to(point(px(frame.unit * 0.95), px(frame.unit * 0.95)), px(0.), true, true, frame.at(0.4, 0.85));
    if let Ok(path) = arc.build() {
        window.paint_path(path, ink.color);
    }
    fill(&frame.polygon(&[(0.25, 0.55), (0.75, 0.75), (0.3, 1.1)]), ink.color, window);
}

fn collapsed_marker(frame: &Frame, ink: &Ink, window: &mut Window) {
    closed(&frame.polygon(&[(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]), ink, window);
    line(&[frame.at(0., -0.6), frame.at(0., 0.6)], 1.4, ink, window);
    line(&[frame.at(-0.6, 0.), frame.at(0.6, 0.)], 1.4, ink, window);
}

fn line(points: &[Point<Pixels>], width: f32, ink: &Ink, window: &mut Window) {
    let mut path = PathBuilder::stroke(px(width * ink.zoom.max(0.5)));
    path.add_polygon(points, false);
    if let Ok(path) = path.build() {
        window.paint_path(path, ink.color);
    }
}

fn closed(points: &[Point<Pixels>], ink: &Ink, window: &mut Window) {
    let mut path = PathBuilder::stroke(px(1.2 * ink.zoom.max(0.5)));
    path.add_polygon(points, true);
    if let Ok(path) = path.build() {
        window.paint_path(path, ink.color);
    }
}

fn fill(points: &[Point<Pixels>], color: Hsla, window: &mut Window) {
    let mut path = PathBuilder::fill();
    path.add_polygon(points, true);
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn shape(points: &[Point<Pixels>], filled: bool, ink: &Ink, window: &mut Window) {
    if filled {
        fill(points, ink.color, window);
    } else {
        closed(points, ink, window);
    }
}
