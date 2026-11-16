use camunda_core::models::{BpmnDiagram, BpmnNode};
use gpui_kit::*;

use super::BpmnView;
use super::kind::is_container;

const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 4.0;
const FIT_PADDING: f32 = 40.;

impl BpmnView {
    // Пока пользователь не двигал и не масштабировал, схема вписана в окно.
    pub(super) fn current(&self) -> (f32, Point<Pixels>) {
        self.transform.unwrap_or_else(|| fit(&self.diagram, self.viewport.get()))
    }

    pub(super) fn zoom_at(&mut self, factor: f32, anchor: Point<Pixels>, cx: &mut Context<Self>) {
        let (zoom, offset) = self.current();
        let next = (zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let ratio = next / zoom;
        let origin = self.viewport.get().origin;
        let local = anchor - origin;
        let offset = point(local.x - (local.x - offset.x) * ratio, local.y - (local.y - offset.y) * ratio);
        self.transform = Some((next, offset));
        cx.notify();
    }

    pub(super) fn pan(&mut self, delta: Point<Pixels>, cx: &mut Context<Self>) {
        let (zoom, offset) = self.current();
        self.transform = Some((zoom, offset + delta));
        cx.notify();
    }

    pub(super) fn node_at(&self, position: Point<Pixels>) -> Option<&BpmnNode> {
        let (zoom, offset) = self.current();
        let local = position - self.viewport.get().origin;
        let x = (f32::from(local.x) - f32::from(offset.x)) / zoom;
        let y = (f32::from(local.y) - f32::from(offset.y)) / zoom;
        self.diagram
            .nodes
            .iter()
            .rev()
            .filter(|node| !is_container(node))
            .find(|node| x >= node.x && x <= node.x + node.width && y >= node.y && y <= node.y + node.height)
    }

    pub(super) fn center_of(&self) -> Point<Pixels> {
        self.viewport.get().center()
    }
}

// Масштаб и сдвиг, при которых вся схема помещается в область с отступом.
pub(super) fn fit(diagram: &BpmnDiagram, viewport: Bounds<Pixels>) -> (f32, Point<Pixels>) {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for node in &diagram.nodes {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    if min_x > max_x || viewport.size.width <= px(0.) {
        return (1., point(px(FIT_PADDING), px(FIT_PADDING)));
    }
    let (width, height) = (f32::from(viewport.size.width) - FIT_PADDING * 2., f32::from(viewport.size.height) - FIT_PADDING * 2.);
    let zoom = (width / (max_x - min_x)).min(height / (max_y - min_y)).clamp(MIN_ZOOM, 1.5);
    let x = (f32::from(viewport.size.width) - (max_x - min_x) * zoom) / 2. - min_x * zoom;
    let y = (f32::from(viewport.size.height) - (max_y - min_y) * zoom) / 2. - min_y * zoom;
    (zoom, point(px(x), px(y)))
}
