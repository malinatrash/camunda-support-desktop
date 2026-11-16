use camunda_core::models::BpmnNode;
use gpui_kit::*;

use super::palette::Palette;

pub struct Stage<'a> {
    pub bounds: Bounds<Pixels>,
    pub zoom: f32,
    pub offset: Point<Pixels>,
    pub visible: Bounds<Pixels>,
    pub palette: &'a Palette,
}

impl<'a> Stage<'a> {
    pub fn new(bounds: Bounds<Pixels>, zoom: f32, offset: Point<Pixels>, palette: &'a Palette) -> Self {
        Self { bounds, zoom, offset, visible: bounds.dilate(px(60. * zoom.max(1.))), palette }
    }

    pub fn to_screen(&self, x: f32, y: f32) -> Point<Pixels> {
        self.bounds.origin + self.offset + point(px(x * self.zoom), px(y * self.zoom))
    }

    pub fn rect(&self, node: &BpmnNode) -> Bounds<Pixels> {
        Bounds::new(self.to_screen(node.x, node.y), size(px(node.width * self.zoom), px(node.height * self.zoom)))
    }
}
