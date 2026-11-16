use gpui_kit::*;

// Точки сетки не гуще 24 px на экране, иначе на мелком масштабе их десятки тысяч.
pub fn paint_grid(bounds: Bounds<Pixels>, zoom: f32, offset: Point<Pixels>, color: Hsla, window: &mut Window) {
    let mut step = 40. * zoom;
    while step < 24. {
        step *= 2.;
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
