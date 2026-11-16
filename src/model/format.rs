// Число с узким пробелом между тысячами.
pub fn grouped(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut result = String::new();
    for (index, c) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            result.push('\u{202f}');
        }
        result.push(c);
    }
    if value < 0 { format!("-{result}") } else { result }
}

pub fn format_duration(millis: i64) -> String {
    let minutes = millis / 60_000;
    let (days, hours, rest) = (minutes / (24 * 60), (minutes % (24 * 60)) / 60, minutes % 60);
    if days > 0 {
        format!("{days} д. {hours} ч.")
    } else if hours > 0 {
        format!("{hours} ч. {rest} мин.")
    } else {
        format!("{rest} мин.")
    }
}

// «2026-10-30 12:00» из метки времени Camunda.
pub fn short_time(timestamp: Option<&str>) -> String {
    timestamp.map(|value| value.chars().take(16).collect::<String>().replace('T', " ")).unwrap_or_else(|| "—".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_splits_thousands() {
        assert_eq!(grouped(1421), "1\u{202f}421");
        assert_eq!(grouped(999), "999");
    }
}
