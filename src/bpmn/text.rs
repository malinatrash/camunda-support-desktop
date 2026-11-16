use gpui_kit::*;

use super::labels::LABEL_LINE;

// Подпись внутри задачи или под событием и шлюзом.
pub fn paint_label(
    lines: &[SharedString],
    rect: Bounds<Pixels>,
    inside: bool,
    font_size: Pixels,
    color: Hsla,
    window: &mut Window,
    cx: &mut App,
) {
    let line_height = font_size * LABEL_LINE;
    let block = line_height * lines.len() as f32;
    let top = if inside { rect.center().y - block / 2. } else { rect.bottom() + px(3.) };
    for (index, line) in lines.iter().enumerate() {
        let area = Bounds::new(point(rect.left(), top + line_height * index as f32), size(rect.size.width, line_height));
        paint_text(line, area, font_size, color, window, cx);
    }
}

// Одна строка по центру области.
pub fn paint_text(text: &SharedString, area: Bounds<Pixels>, font_size: Pixels, color: Hsla, window: &mut Window, cx: &mut App) {
    let run =
        TextRun { len: text.len(), font: window.text_style().font(), color, background_color: None, underline: None, strikethrough: None };
    let line = window.text_system().shape_line(text.clone(), font_size, &[run], None);
    let line_height = font_size * LABEL_LINE;
    let origin = point(area.center().x - line.width / 2., area.center().y - line_height / 2.);
    line.paint(origin, line_height, TextAlign::Left, None, window, cx).ok();
}
