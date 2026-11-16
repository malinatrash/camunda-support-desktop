mod camera;
mod containers;
mod edges;
mod grid;
mod kind;
mod labels;
mod markers;
mod nodes;
mod paint;
mod palette;
mod render;
mod shapes;
mod stage;
mod text;
mod toolbar;
mod wrap;

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use camunda_core::models::BpmnDiagram;
use gpui_kit::*;

use labels::LabelCache;

#[derive(Default)]
pub struct Highlights {
    pub active: HashSet<String>,
    pub incidents: HashMap<String, usize>,
    pub completed: HashSet<String>,
}

pub struct SelectNode(pub String);

pub struct BpmnView {
    diagram: Arc<BpmnDiagram>,
    highlights: Arc<Highlights>,
    selected: Option<String>,
    transform: Option<(f32, Point<Pixels>)>,
    viewport: Rc<Cell<Bounds<Pixels>>>,
    labels: Rc<LabelCache>,
    drag: Option<(Point<Pixels>, Point<Pixels>)>,
    moved: bool,
}

impl EventEmitter<SelectNode> for BpmnView {}

impl BpmnView {
    pub fn new(diagram: BpmnDiagram, highlights: Highlights) -> Self {
        Self {
            diagram: Arc::new(diagram),
            highlights: Arc::new(highlights),
            selected: None,
            transform: None,
            viewport: Rc::new(Cell::new(Bounds::default())),
            labels: Rc::default(),
            drag: None,
            moved: false,
        }
    }

    // Новые подсветки после обновления заявки без сброса зума и сдвига.
    pub fn update(&mut self, diagram: &BpmnDiagram, highlights: Highlights, cx: &mut Context<Self>) {
        if *self.diagram != *diagram {
            self.diagram = Arc::new(diagram.clone());
            self.labels.borrow_mut().clear();
            self.transform = None;
        }
        self.highlights = Arc::new(highlights);
        cx.notify();
    }
}
