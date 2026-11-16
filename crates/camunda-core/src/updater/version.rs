// Сравнивает версии вида v1.2.3 / 1.2.3-dev.4: true, если candidate новее current.
pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    fn parts(value: &str) -> Vec<i64> {
        value
            .trim()
            .trim_start_matches('v')
            .split('-')
            .next()
            .unwrap_or("")
            .split('+')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|part| part.parse::<i64>().unwrap_or(0))
            .collect()
    }
    let candidate = parts(candidate);
    let current = parts(current);
    let size = candidate.len().max(current.len()).max(3);
    for index in 0..size {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = current.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semantic_versions() {
        assert!(is_newer_version("v1.5.0", "1.4.5"));
        assert!(is_newer_version("2.0.0", "1.9.9-dev.3+abc1234"));
        assert!(is_newer_version("v1.2.0", "1.1.9"));
        assert!(is_newer_version("2.0", "1.99.99"));
        assert!(!is_newer_version("1.4.5", "1.4.5"));
        assert!(!is_newer_version("v1.4.4", "1.4.5"));
        assert!(!is_newer_version("1.1.9", "1.2.0"));
        assert!(is_newer_version("1.4.5.1", "1.4.5"));
    }
}
