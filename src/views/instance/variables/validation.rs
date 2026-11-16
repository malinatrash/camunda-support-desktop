// Подсказка по формату значения для типа переменной.
pub(super) fn hint(value_type: &str) -> String {
    match value_type.to_lowercase().as_str() {
        "boolean" => tr!("instance.variables.hint_boolean").into(),
        "short" | "integer" | "long" => tr!("instance.variables.hint_integer").into(),
        "double" => tr!("instance.variables.hint_double").into(),
        "date" => tr!("instance.variables.hint_date").into(),
        "json" => tr!("instance.variables.hint_json").into(),
        _ => tr!("instance.variables.hint_other", kind = value_type).into(),
    }
}

pub(super) fn validate(value_type: &str, value: &str) -> Option<String> {
    let value = value.trim();
    match value_type.to_lowercase().as_str() {
        "boolean" => (!value.eq_ignore_ascii_case("true") && !value.eq_ignore_ascii_case("false"))
            .then(|| tr!("instance.variables.error_boolean").into()),
        "short" => value.parse::<i16>().is_err().then(|| tr!("instance.variables.error_range", kind = "Short").into()),
        "integer" => value.parse::<i32>().is_err().then(|| tr!("instance.variables.error_range", kind = "Integer").into()),
        "long" => value.parse::<i64>().is_err().then(|| tr!("instance.variables.error_range", kind = "Long").into()),
        "double" => (!value.parse::<f64>().is_ok_and(f64::is_finite)).then(|| tr!("instance.variables.error_double").into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn validates_numbers_and_booleans() {
        assert!(validate("Integer", "42").is_none());
        assert!(validate("Integer", "4 2").is_some());
        assert!(validate("Boolean", "TRUE").is_none());
        assert!(validate("Short", "40000").is_some());
        assert!(validate("Double", "1.5").is_none());
        assert!(validate("String", "что угодно").is_none());
    }
}
