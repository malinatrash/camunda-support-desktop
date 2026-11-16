use gpui_kit::*;

use super::BpmnView;
use super::SelectNode;
use super::camera::fit;
use super::paint::paint_diagram;
use super::palette::palette;
use super::stage::Stage;
use super::toolbar::toolbar;

impl Render for BpmnView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = palette(cx);
        let background = palette.background;
        let diagram = self.diagram.clone();
        let fit_diagram = self.diagram.clone();
        let highlights = self.highlights.clone();
        let selected = self.selected.clone();
        let transform = self.transform;
        let viewport = self.viewport.clone();
        let labels = self.labels.clone();
        let zoom_label = format!("{}%", (self.current().0 * 100.).round());

        div()
            .id("bpmn")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(background)
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
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.drag = Some((event.position, this.current().1));
                    this.moved = false;
                    cx.notify();
                }),
            )
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
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    if !this.moved {
                        let id = this.node_at(event.position).map(|node| node.id.clone());
                        if let Some(id) = &id {
                            cx.emit(SelectNode(id.clone()));
                        }
                        this.selected = id;
                    }
                    this.drag = None;
                    cx.notify();
                }),
            )
            .child(
                canvas(
                    move |bounds, window, _| {
                        if viewport.replace(bounds) != bounds {
                            window.request_animation_frame();
                        }
                        transform.unwrap_or_else(|| fit(&fit_diagram, bounds))
                    },
                    move |bounds, (zoom, offset), window, cx| {
                        let stage = Stage::new(bounds, zoom, offset, &palette);
                        paint_diagram(&diagram, &highlights, &labels, selected.as_deref(), &stage, window, cx);
                    },
                )
                .size_full(),
            )
            .child(toolbar(zoom_label, cx))
    }
}
