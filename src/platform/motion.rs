use std::time::Duration;

use gpui_kit::*;

const ENTER: Duration = Duration::from_millis(160);
const PAGE: Duration = Duration::from_millis(260);
const STAGGER_STEP: f32 = 0.035;
const STAGGER_ROW: f32 = 0.22;
const STAGGER_LIMIT: usize = 12;
const PULSE: Duration = Duration::from_millis(1600);
const COUNT: Duration = Duration::from_millis(700);
const TRAVEL: f32 = 6.;

// Появление всплывающего окна: проявляется и съезжает на место за 160 мс; generation меняется на каждое открытие.
pub fn enter<E: IntoElement + Styled + 'static>(id: &'static str, generation: usize, element: E) -> impl IntoElement {
    element.with_animation(
        ElementId::NamedInteger(id.into(), generation as u64),
        Animation::new(ENTER).with_easing(ease_out_quint()),
        |element, delta| element.mt(px(TRAVEL * (delta - 1.))).opacity(delta),
    )
}

// Смена экрана: новый экран проявляется и поднимается снизу; route меняется на каждый переход.
pub fn page<E: IntoElement + Styled + 'static>(route: usize, element: E) -> impl IntoElement {
    element.with_animation(
        ElementId::NamedInteger("page".into(), route as u64),
        Animation::new(PAGE).with_easing(ease_out_quint()),
        |element, delta| element.top(px(2. * TRAVEL * (1. - delta))).opacity(delta),
    )
}

// Строки списка появляются лесенкой: каждая следующая чуть позже, после двенадцатой без задержки.
pub fn stagger<E: IntoElement + Styled + 'static>(id: impl Into<ElementId>, index: usize, element: E) -> impl IntoElement {
    let delay = STAGGER_STEP * index.min(STAGGER_LIMIT) as f32;
    let total = delay + STAGGER_ROW;
    let easing = ease_out_quint();
    element.with_animation(
        id,
        Animation::new(Duration::from_secs_f32(total)).with_easing(move |t| easing(((t * total - delay) / STAGGER_ROW).clamp(0., 1.))),
        |element, delta| element.ml(px(TRAVEL * (delta - 1.))).opacity(delta),
    )
}

// Мягкая пульсация живого индикатора.
pub fn pulse<E: IntoElement + Styled + 'static>(id: impl Into<ElementId>, element: E) -> impl IntoElement {
    element.with_animation(id, Animation::new(PULSE).repeat().with_easing(pulsating_between(0.35, 1.)), |element, delta| {
        element.opacity(delta)
    })
}

// Число набегает от нуля до значения; key меняется, когда значение нужно проиграть заново.
pub fn count_up(id: impl Into<ElementId>, value: i64, format: impl Fn(i64) -> String + 'static) -> impl IntoElement {
    div().with_animation(id, Animation::new(COUNT).with_easing(ease_out_quint()), move |element, delta| {
        element.child(format((value as f32 * delta).round() as i64))
    })
}
