use std::collections::HashMap;

#[macro_export]
macro_rules! tr {
    ($translations:expr, $key:expr) => {
        tr_impl($translations, $key, &[])
    };
    ($translations:expr, $key:expr, $( $k:expr => $v:expr ),* ) => {
        tr_impl($translations, $key, &[ $( ($k, $v) ),* ])
    };
}

pub fn tr_impl(
    translations: &HashMap<String, String>,
    key: &str,
    params: &[(&str, &str)],
) -> String {
    let translation_value = translations.get(key);
    if params.is_empty() {
        return translation_value
            .cloned()
            .unwrap_or_else(|| key.to_string());
    }

    let text = translation_value.map_or(key, |value| value.as_str());
    let mut result = text.to_string();
    for (k, v) in params {
        result = result.replace(&format!("{{{}}}", k), v);
    }
    result
}
