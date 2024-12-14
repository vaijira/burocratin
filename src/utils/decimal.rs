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

pub fn valid_str_number_with_decimals(number: &str, decimal_number: u16, locale: &Locale) -> bool {
    let mut state = 0; // 0 integer part, 1 decimal part
    let mut decimals = 0;
    for c in number.chars() {
        if c.is_numeric() {
            if state == 1 {
                decimals += 1;
                if decimals > decimal_number {
                    return false;
                }
            }
        } else if state == 0 {
            if locale.decimal().starts_with(c) || c == '.' {
                state = 1;
            } else {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_to_str_locale() {
        let x = Decimal::new(2314, 2);
        assert_eq!("23,14", decimal_to_str_locale(&x, &Locale::es));
    }

    #[test]
    fn test_valid_str_number_with_decimals() {
        assert!(valid_str_number_with_decimals("23,14", 2, &Locale::es));
        assert!(valid_str_number_with_decimals("2333.14", 2, &Locale::es));
        assert_eq!(
            false,
            valid_str_number_with_decimals("323,143", 2, &Locale::es)
        );
        assert_eq!(
            false,
            valid_str_number_with_decimals("423h14", 2, &Locale::es)
        );
        assert_eq!(
            false,
            valid_str_number_with_decimals("5a23.14", 2, &Locale::es)
        );
    }
}
