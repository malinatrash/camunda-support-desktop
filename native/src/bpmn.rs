use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use camunda_core::models::{BpmnDiagram, BpmnNode};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, IconName, Sizable, h_flex};
use gpui_kit::*;

const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 4.0;
const FIT_PADDING: f32 = 40.;

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
    // None — схема вписана в окно; Some — пользователь двигал или масштабировал.
    transform: Option<(f32, Point<Pixels>)>,
    viewport: Rc<Cell<Bounds<Pixels>>>,
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
            drag: None,
            moved: false,
        }
    }

    // Новые подсветки после обновления заявки без сброса зума и сдвига.
    pub fn update(&mut self, diagram: &BpmnDiagram, highlights: Highlights, cx: &mut Context<Self>) {
        if *self.diagram != *diagram {
            self.diagram = Arc::new(diagram.clone());
            self.transform = None;
        }
        self.highlights = Arc::new(highlights);
        cx.notify();
    }

    fn current(&self) -> (f32, Point<Pixels>) {
        self.transform.unwrap_or_else(|| fit(&self.diagram, self.viewport.get()))
    }

    fn zoom_at(&mut self, factor: f32, anchor: Point<Pixels>, cx: &mut Context<Self>) {
        let (zoom, offset) = self.current();
        let next = (zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let ratio = next / zoom;
        let origin = self.viewport.get().origin;
        let local = anchor - origin;
        let offset = point(local.x - (local.x - offset.x) * ratio, local.y - (local.y - offset.y) * ratio);
        self.transform = Some((next, offset));
        cx.notify();
    }

    fn pan(&mut self, delta: Point<Pixels>, cx: &mut Context<Self>) {
        let (zoom, offset) = self.current();
        self.transform = Some((zoom, offset + delta));
        cx.notify();
    }

    fn node_at(&self, position: Point<Pixels>) -> Option<&BpmnNode> {
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

    fn center_of(&self) -> Point<Pixels> {
        self.viewport.get().center()
    }
}

// Масштаб и сдвиг, при которых вся схема помещается в область с отступом.
fn fit(diagram: &BpmnDiagram, viewport: Bounds<Pixels>) -> (f32, Point<Pixels>) {
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

fn is_container(node: &BpmnNode) -> bool {
    matches!(node.node_type.as_str(), "participant" | "lane" | "subProcess" | "transaction")
}

fn is_event(node: &BpmnNode) -> bool {
    node.node_type.ends_with("Event")
}

fn is_gateway(node: &BpmnNode) -> bool {
    node.node_type.ends_with("Gateway")
}

struct Palette {
    background: Hsla,
    grid: Hsla,
    stroke: Hsla,
    text: Hsla,
    active: Hsla,
    danger: Hsla,
    success: Hsla,
}


impl Render for BpmnView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let palette = Palette {
            background: theme.background,
            grid: theme.border.opacity(0.35),
            stroke: theme.foreground.opacity(0.75),
            text: theme.foreground,
            active: theme.blue,
            danger: theme.danger,
            success: theme.success,
        };
        let diagram = self.diagram.clone();
        let fit_diagram = self.diagram.clone();
        let highlights = self.highlights.clone();
        let selected = self.selected.clone();
        let transform = self.transform;
        let viewport = self.viewport.clone();
        let zoom_label = format!("{}%", (self.current().0 * 100.).round());

        div()
            .id("bpmn")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(palette.background)
            .cursor(if self.drag.is_some() { CursorStyle::ClosedHand } else { CursorStyle::Arrow })
            .on_scroll(cx.listener(|this, event: &ScrollEvent, window, cx| {
                let delta = event.delta.pixel_delta(window.line_height());
                if event.modifiers.platform || event.modifiers.control {
                    this.zoom_at((1. + f32::from(delta.y) / 300.).max(0.1), event.position, cx);
                } else {
                    this.pan(delta, cx);
                }
            }))
            .on_pinch(cx.listener(|this, event: &PinchEvent, _, cx| this.zoom_at(1. + event.delta, event.position, cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.drag = Some((event.position, this.current().1));
                this.moved = false;
                cx.notify();
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                let Some((start, offset)) = this.drag else { return };
                let delta = event.position - start;
                if f32::from(delta.x).abs() + f32::from(delta.y).abs() > 3. {
                    this.moved = true;
                }
                if this.moved {
                    let zoom = this.current().0;
                    this.transform = Some((zoom, offset + delta));
                    cx.notify();
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, event: &MouseUpEvent, _, cx| {
                if !this.moved {
                    let id = this.node_at(event.position).map(|node| node.id.clone());
                    if let Some(id) = &id {
                        cx.emit(SelectNode(id.clone()));
                    }
                    this.selected = id;
                }
                this.drag = None;
                cx.notify();
            }))
            .child(
                canvas(
                    move |bounds, _, _| {
                        viewport.set(bounds);
                        transform.unwrap_or_else(|| fit(&fit_diagram, bounds))
                    },
                    move |bounds, (zoom, offset), window, cx| {
                        paint_diagram(&diagram, &highlights, selected.as_deref(), bounds, zoom, offset, &palette, window, cx);
                    },
                )
                .size_full(),
            )
            .child(
                h_flex()
                    .absolute()
                    .top_2()
                    .right_2()
                    .gap_1()
                    .p_1()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().popover)
                    .child(Button::new("zoom-out").icon(IconName::Minus).ghost().xsmall().on_click(cx.listener(|this, _, _, cx| {
                        let center = this.center_of();
                        this.zoom_at(0.8, center, cx)
                    })))
                    .child(
                        Button::new("fit").label("По размеру").ghost().xsmall().on_click(cx.listener(|this, _, _, cx| {
                            this.transform = None;
                            cx.notify();
                        })),
                    )
                    .child(Button::new("zoom-in").icon(IconName::Plus).ghost().xsmall().on_click(cx.listener(|this, _, _, cx| {
                        let center = this.center_of();
                        this.zoom_at(1.25, center, cx)
                    })))
                    .child(div().w(px(44.)).text_xs().text_right().text_color(cx.theme().muted_foreground).child(zoom_label)),
            )
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_diagram(
    diagram: &BpmnDiagram,
    highlights: &Highlights,
    selected: Option<&str>,
    bounds: Bounds<Pixels>,
    zoom: f32,
    offset: Point<Pixels>,
    palette: &Palette,
    window: &mut Window,
    cx: &mut App,
) {
    let to_screen = |x: f32, y: f32| bounds.origin + offset + point(px(x * zoom), px(y * zoom));
    window.paint_quad(fill(bounds, palette.background));
    paint_grid(bounds, zoom, offset, palette.grid, window);

    for node in diagram.nodes.iter().filter(|node| is_container(node)) {
        let rect = Bounds::new(to_screen(node.x, node.y), size(px(node.width * zoom), px(node.height * zoom)));
        window.paint_quad(quad(rect, px(6. * zoom), transparent_black(), px(1.), palette.stroke.opacity(0.35), BorderStyle::Dashed));
    }

    for edge in &diagram.edges {
        let points: Vec<Point<Pixels>> = edge.waypoints.iter().map(|p| to_screen(p.x, p.y)).collect();
        if points.len() < 2 {
            continue;
        }
        let mut path = PathBuilder::stroke(px((1.4 * zoom).max(1.)));
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, palette.stroke);
        }
        paint_arrow(points[points.len() - 2], points[points.len() - 1], zoom, palette.stroke, window);
    }

    let font_size = px((12. * zoom).clamp(4., 20.));
    for node in diagram.nodes.iter().filter(|node| !is_container(node)) {
        let rect = Bounds::new(to_screen(node.x, node.y), size(px(node.width * zoom), px(node.height * zoom)));
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
        let border = accent.unwrap_or(palette.stroke);
        let background = accent.map(|color| color.opacity(0.12)).unwrap_or(palette.background);
        let width = px(if accent.is_some() { 2.4 } else { 1.4 } * zoom.max(0.6));
        if is_event(node) {
            let border_width = if node.node_type == "endEvent" { width * 2. } else { width };
            window.paint_quad(quad(rect, rect.size.width / 2., background, border_width, border, BorderStyle::Solid));
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
        } else {
            window.paint_quad(quad(rect, px(8. * zoom), background, width, border, BorderStyle::Solid));
        }
        if selected == Some(node.id.as_str()) {
            let ring = rect.dilate(px(4.));
            window.paint_quad(quad(ring, px(10. * zoom), transparent_black(), px(2.), palette.active, BorderStyle::Dashed));
        }
        if let Some(name) = node.name.as_deref().filter(|name| !name.trim().is_empty()) {
            let inside = !is_event(node) && !is_gateway(node);
            paint_label(name, rect, inside, font_size, palette.text, window, cx);
        }
        if incidents > 0 {
            let badge = Bounds::centered_at(point(rect.right(), rect.top()), size(px(18.), px(18.)));
            window.paint_quad(quad(badge, px(9.), palette.danger, px(0.), transparent_black(), BorderStyle::Solid));
            paint_text(&incidents.to_string(), badge, px(11.), white(), window, cx);
        }
    }
}

fn paint_grid(bounds: Bounds<Pixels>, zoom: f32, offset: Point<Pixels>, color: Hsla, window: &mut Window) {
    let step = 40. * zoom;
    if step < 8. {
        return;
    }
    let start_x = f32::from(offset.x).rem_euclid(step);
    let start_y = f32::from(offset.y).rem_euclid(step);
    let dot = size(px(1.5), px(1.5));
    let mut y = start_y;
    while y < f32::from(bounds.size.height) {
        let mut x = start_x;
        while x < f32::from(bounds.size.width) {
            window.paint_quad(fill(Bounds::new(bounds.origin + point(px(x), px(y)), dot), color));
            x += step;
        }
        y += step;
    }
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

// Подпись внутри задачи или под событием и шлюзом: до трёх строк по словам, хвост с многоточием.
fn paint_label(name: &str, rect: Bounds<Pixels>, inside: bool, font_size: Pixels, color: Hsla, window: &mut Window, cx: &mut App) {
    if font_size < px(5.) {
        return;
    }
    let max_width = if inside { rect.size.width - px(10.) } else { rect.size.width * 2.6 };
    let max_lines = if inside { ((rect.size.height - px(6.)) / (font_size * 1.25)).floor().clamp(1., 3.) as usize } else { 2 };
    let lines = wrap_text(name, max_width, max_lines, font_size, window);
    let line_height = font_size * 1.25;
    let block = line_height * lines.len() as f32;
    let top = if inside { rect.center().y - block / 2. } else { rect.bottom() + px(3.) };
    for (index, line) in lines.iter().enumerate() {
        let area = Bounds::new(point(rect.center().x - max_width / 2., top + line_height * index as f32), size(max_width, line_height));
        paint_text(line, area, font_size, color, window, cx);
    }
}

fn wrap_text(name: &str, max_width: Pixels, max_lines: usize, font_size: Pixels, window: &mut Window) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let words: Vec<&str> = name.split_whitespace().collect();
    for (index, word) in words.iter().enumerate() {
        let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
        if current.is_empty() || shaped_width(&candidate, font_size, window) <= max_width {
            current = candidate;
            continue;
        }
        lines.push(std::mem::take(&mut current));
        current = word.to_string();
        if lines.len() == max_lines - 1 {
            current = words[index..].join(" ");
            break;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.truncate(max_lines);
    if let Some(last) = lines.last_mut() {
        *last = ellipsize(last, max_width, font_size, window);
    }
    lines
}

fn ellipsize(text: &str, max_width: Pixels, font_size: Pixels, window: &mut Window) -> String {
    if shaped_width(text, font_size, window) <= max_width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let middle = (low + high + 1) / 2;
        let candidate: String = chars[..middle].iter().collect::<String>() + "…";
        if shaped_width(&candidate, font_size, window) <= max_width { low = middle } else { high = middle - 1 }
    }
    chars[..low].iter().collect::<String>() + "…"
}

fn shaped_width(text: &str, font_size: Pixels, window: &mut Window) -> Pixels {
    let run = TextRun { len: text.len(), font: window.text_style().font(), color: black(), background_color: None, underline: None, strikethrough: None };
    window.text_system().shape_line(SharedString::from(text.to_string()), font_size, &[run], None).width
}

fn paint_text(text: &str, area: Bounds<Pixels>, font_size: Pixels, color: Hsla, window: &mut Window, cx: &mut App) {
    let run = TextRun { len: text.len(), font: window.text_style().font(), color, background_color: None, underline: None, strikethrough: None };
    let line = window.text_system().shape_line(SharedString::from(text.to_string()), font_size, &[run], None);
    let line_height = font_size * 1.3;
    let origin = point(area.center().x - line.width / 2., area.center().y - line_height / 2.);
    line.paint(origin, line_height, TextAlign::Left, None, window, cx).ok();
}
