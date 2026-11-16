// Подсказка по формату значения для типа переменной.
pub(super) fn hint(value_type: &str) -> String {
    match value_type.to_lowercase().as_str() {
        "boolean" => "Введите true или false.".into(),
        "short" | "integer" | "long" => "Введите целое число без пробелов и разделителей.".into(),
        "double" => "Введите число, дробная часть отделяется точкой.".into(),
        "date" => "Сохранится тип Date; используйте формат даты Camunda с часовым поясом.".into(),
        "json" => "Введите полный корректный JSON. Он сохранится строковым JSON-значением Camunda.".into(),
        _ => format!("Тип переменной останется {value_type}."),
    }
}

pub(super) fn validate(value_type: &str, value: &str) -> Option<String> {
    let value = value.trim();
    match value_type.to_lowercase().as_str() {
        "boolean" => {
            (!value.eq_ignore_ascii_case("true") && !value.eq_ignore_ascii_case("false")).then(|| "Допустимы только true или false.".into())
        }
        "short" => value.parse::<i16>().is_err().then(|| "Значение выходит за диапазон Short или не является целым числом.".into()),
        "integer" => value.parse::<i32>().is_err().then(|| "Значение выходит за диапазон Integer или не является целым числом.".into()),
        "long" => value.parse::<i64>().is_err().then(|| "Значение выходит за диапазон Long или не является целым числом.".into()),
        "double" => (!value.parse::<f64>().is_ok_and(f64::is_finite)).then(|| "Введите конечное число в формате Double.".into()),
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
