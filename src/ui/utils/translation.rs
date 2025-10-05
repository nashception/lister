#[macro_export]
macro_rules! tr {
    ($translations:expr, $key:expr) => {{
        let key = $key;
        $translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }};
    ($translations:expr, $key:expr, $( $k:expr => $v:expr ),* ) => {{
        let key = $key;
        let text = $translations
            .get(key)
            .map_or(key, |value| value.as_str());
        let mut result = text.to_string();
        $(
            result = result.replace(&format!("{{{}}}", $k), $v);
        )*
        result
    }};
}
