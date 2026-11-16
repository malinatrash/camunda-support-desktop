use camunda_core::models::BpmnDiagram;
use gpui_kit::*;

use super::Highlights;
use super::containers::paint_containers;
use super::edges::paint_edges;
use super::grid::paint_grid;
use super::labels::LabelCache;
use super::nodes::paint_nodes;
use super::stage::Stage;

// Кадр схемы: фон, сетка, контейнеры, рёбра и узлы поверх.
pub fn paint_diagram(
    diagram: &BpmnDiagram,
    highlights: &Highlights,
    labels: &LabelCache,
    selected: Option<&str>,
    stage: &Stage,
    window: &mut Window,
    cx: &mut App,
) {
    window.paint_quad(fill(stage.bounds, stage.palette.background));
    paint_grid(stage.bounds, stage.zoom, stage.offset, stage.palette.grid, window);
    paint_containers(diagram, stage, window);
    paint_edges(diagram, stage, window);
    paint_nodes(diagram, highlights, labels, selected, stage, window, cx);
}
