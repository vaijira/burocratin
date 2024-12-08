use num_format::Locale;
use rust_decimal::Decimal;

pub fn transform_i18n_es_str(input: &str) -> String {
    str::replace(&str::replace(input, ".", ""), ",", ".")
}

pub fn normalize_str(input: &str) -> String {
    str::replace(input, ",", "")
}

pub fn decimal_to_str_locale(number: &Decimal, locale: &Locale) -> String {
    let mut result = number.to_string();
    if let Some(idx) = result.rfind('.') {
        result.replace_range(idx..idx + 1, locale.decimal());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_to_str_locale() {
        let x = Decimal::new(2314, 2);
        assert_eq!("23,14", decimal_to_str_locale(&x, &Locale::es));
    }
}
