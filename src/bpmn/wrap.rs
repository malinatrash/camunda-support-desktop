use gpui_kit::*;

// До max_lines строк по словам, хвост последней строки с многоточием.
pub fn wrap_text(name: &str, max_width: Pixels, max_lines: usize, font_size: Pixels, window: &mut Window) -> Vec<SharedString> {
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
    lines.into_iter().map(SharedString::from).collect()
}

fn ellipsize(text: &str, max_width: Pixels, font_size: Pixels, window: &mut Window) -> String {
    if shaped_width(text, font_size, window) <= max_width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        let candidate: String = chars[..middle].iter().collect::<String>() + "…";
        if shaped_width(&candidate, font_size, window) <= max_width { low = middle } else { high = middle - 1 }
    }
    chars[..low].iter().collect::<String>() + "…"
}

fn shaped_width(text: &str, font_size: Pixels, window: &mut Window) -> Pixels {
    let run = TextRun {
        len: text.len(),
        font: window.text_style().font(),
        color: black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window.text_system().shape_line(SharedString::from(text.to_string()), font_size, &[run], None).width
}
