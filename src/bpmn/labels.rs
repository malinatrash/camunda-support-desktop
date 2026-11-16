use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use camunda_core::models::{BpmnDiagram, BpmnNode};
use gpui_kit::*;

use super::kind::{is_container, is_event, is_gateway};
use super::wrap::wrap_text;

pub const LABEL_FONT: f32 = 12.;
pub const LABEL_LINE: f32 = 1.25;

// Строки подписей раскладываются один раз в координатах схемы: зум и сдвиг их не пересчитывают.
pub type LabelCache = RefCell<HashMap<String, Rc<[SharedString]>>>;

// Раскладка всех подписей на первом кадре после смены схемы.
pub fn prepare_labels(diagram: &BpmnDiagram, labels: &LabelCache, window: &mut Window) {
    if labels.borrow().is_empty() {
        for node in diagram.nodes.iter().filter(|node| !is_container(node)) {
            label_lines(node, labels, window);
        }
    }
}

pub fn label_lines(node: &BpmnNode, labels: &LabelCache, window: &mut Window) -> Rc<[SharedString]> {
    if let Some(lines) = labels.borrow().get(&node.id) {
        return lines.clone();
    }
    let name = node.name.as_deref().map(str::trim).unwrap_or_default();
    let inside = !is_event(node) && !is_gateway(node);
    let max_width = px(if inside { node.width - 10. } else { node.width * 2.6 });
    let max_lines = if inside { ((node.height - 6.) / (LABEL_FONT * LABEL_LINE)).floor().clamp(1., 3.) as usize } else { 2 };
    let lines: Rc<[SharedString]> =
        if name.is_empty() { Rc::from([]) } else { wrap_text(name, max_width, max_lines, px(LABEL_FONT), window).into() };
    labels.borrow_mut().insert(node.id.clone(), lines.clone());
    lines
}
