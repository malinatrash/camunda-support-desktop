use camunda_core::models::BpmnDiagram;
use gpui_kit::*;

use super::stage::Stage;

// Потоки со стрелкой на конце, невидимые пропускаются.
pub fn paint_edges(diagram: &BpmnDiagram, stage: &Stage, window: &mut Window) {
    let (zoom, color) = (stage.zoom, stage.palette.stroke);
    for edge in &diagram.edges {
        let points: Vec<Point<Pixels>> = edge.waypoints.iter().map(|p| stage.to_screen(p.x, p.y)).collect();
        if points.len() < 2 || !stage.visible.intersects(&enclosing(&points)) {
            continue;
        }
        let mut path = PathBuilder::stroke(px((1.4 * zoom).max(1.)));
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
        paint_arrow(points[points.len() - 2], points[points.len() - 1], zoom, color, window);
    }
}

fn enclosing(points: &[Point<Pixels>]) -> Bounds<Pixels> {
    let min = points.iter().fold(points[0], |acc, p| point(acc.x.min(p.x), acc.y.min(p.y)));
    let max = points.iter().fold(points[0], |acc, p| point(acc.x.max(p.x), acc.y.max(p.y)));
    Bounds::from_corners(min, point(max.x + px(1.), max.y + px(1.)))
}

fn paint_arrow(from: Point<Pixels>, to: Point<Pixels>, zoom: f32, color: Hsla, window: &mut Window) {
    let (dx, dy) = (f32::from(to.x - from.x), f32::from(to.y - from.y));
    let length = (dx * dx + dy * dy).sqrt();
    if length < 0.01 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    let size = 8. * zoom.max(0.5);
    let base = point(to.x - px(ux * size), to.y - px(uy * size));
    let left = point(base.x - px(uy * size * 0.5), base.y + px(ux * size * 0.5));
    let right = point(base.x + px(uy * size * 0.5), base.y - px(ux * size * 0.5));
    let mut path = PathBuilder::fill();
    path.add_polygon(&[to, left, right], true);
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}
