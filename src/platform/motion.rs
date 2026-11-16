use std::time::Duration;

use gpui_kit::*;

const ENTER: Duration = Duration::from_millis(160);
const TRAVEL: f32 = 6.;

// Появление всплывающего окна: проявляется и съезжает на место за 160 мс; generation меняется на каждое открытие.
pub fn enter<E: IntoElement + Styled + 'static>(id: &'static str, generation: usize, element: E) -> impl IntoElement {
    element.with_animation(
        ElementId::NamedInteger(id.into(), generation as u64),
        Animation::new(ENTER).with_easing(ease_out_quint()),
        |element, delta| element.mt(px(TRAVEL * (delta - 1.))).opacity(delta),
    )
}
