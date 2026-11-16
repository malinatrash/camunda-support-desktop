// Подпись сообщения без идентификаторов и чисел: первая непустая строка, до 140 символов.
pub fn incident_signature(message: Option<&str>) -> String {
    let Some(message) = message else { return rust_i18n::t!("model.no_message").into_owned() };
    let line = message.lines().find(|line| !line.trim().is_empty()).unwrap_or(message);
    let words: Vec<String> = line
        .split_whitespace()
        .map(|word| {
            let hexish = word.len() >= 16 && word.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
            if hexish { "#".to_string() } else { collapse_digits(word) }
        })
        .collect();
    words.join(" ").chars().take(140).collect()
}

fn collapse_digits(word: &str) -> String {
    let mut result = String::with_capacity(word.len());
    let mut in_digits = false;
    for c in word.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                result.push('#');
            }
            in_digits = true;
        } else {
            result.push(c);
            in_digits = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_hides_ids_and_numbers() {
        let message = "Read timed out after 3000 ms for 9d1f6e2a-0001-4bcd-8ef0-123456789abc\nstack";
        assert_eq!(incident_signature(Some(message)), "Read timed out after # ms for #");
    }
}
