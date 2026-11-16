pub(super) struct RawLink<'a> {
    pub(super) scheme: Option<String>,
    pub(super) authority: Option<&'a str>,
    pub(super) path: &'a str,
    pub(super) query: Option<&'a str>,
    pub(super) fragment: Option<&'a str>,
}

// Делит ссылку на части как java.net.URI: пробелы и управляющие символы делают её некорректной.
pub(super) fn split_link(value: &str) -> Option<RawLink<'_>> {
    if value.is_empty() || value.chars().any(|char| char.is_whitespace() || char.is_control()) {
        return None;
    }
    let (without_fragment, fragment) = match value.find('#') {
        Some(index) => (&value[..index], Some(&value[index + 1..])),
        None => (value, None),
    };
    let (without_query, query) = match without_fragment.find('?') {
        Some(index) => (&without_fragment[..index], Some(&without_fragment[index + 1..])),
        None => (without_fragment, None),
    };
    let scheme_end = without_query.find(':').filter(|index| !without_query[..*index].contains('/'));
    let (scheme, rest) = match scheme_end {
        Some(index) if is_valid_scheme(&without_query[..index]) => {
            (Some(without_query[..index].to_lowercase()), &without_query[index + 1..])
        }
        _ => (None, without_query),
    };
    let (authority, path) = match rest.strip_prefix("//") {
        Some(after) => {
            let end = after.find('/').unwrap_or(after.len());
            (Some(&after[..end]), &after[end..])
        }
        None => (None, rest),
    };
    Some(RawLink { scheme, authority, path, query, fragment })
}

pub(super) fn query_value(raw_query: Option<&str>, expected_name: &str) -> Option<String> {
    raw_query?
        .split('&')
        .map(|parameter| match parameter.find('=') {
            Some(index) => (&parameter[..index], &parameter[index + 1..]),
            None => (parameter, ""),
        })
        .find(|(name, _)| decode(name) == expected_name)
        .map(|(_, value)| decode(value))
}

// Декодирует как java.net.URLDecoder: `+` → пробел, %XX → байт, битые последовательности остаются как есть.
pub(super) fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex_pair = (bytes.get(index + 1).and_then(hex_digit), bytes.get(index + 2).and_then(hex_digit));
        match (bytes[index], hex_pair) {
            (b'+', _) => result.push(b' '),
            (b'%', (Some(high), Some(low))) => {
                result.push(high << 4 | low);
                index += 2;
            }
            (byte, _) => result.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn hex_digit(byte: &u8) -> Option<u8> {
    (*byte as char).to_digit(16).map(|digit| digit as u8)
}

fn is_valid_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|char| char.is_ascii_alphanumeric() || matches!(char, '+' | '-' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_percent_sequences_like_java() {
        assert_eq!(decode("a+b%20c%2F"), "a b c/");
        assert_eq!(decode("%D0%B7%D0%B0"), "за");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz"), "%zz");
    }
}
