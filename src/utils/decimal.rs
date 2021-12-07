pub fn transform_i18n_es_str(input: &str) -> String {
    str::replace(&str::replace(input, ".", ""), ",", ".")
}

pub fn normalize_str(input: &str) -> String {
    str::replace(input, ",", "")
}
