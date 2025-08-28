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

pub fn tr_impl(translations: &HashMap<String, String>, key: &str, params: &[(&str, &str)]) -> String {
    let mut text = translations
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string());

    for (k, v) in params {
        text = text.replace(&format!("{{{}}}", k), v);
    }

    text
}