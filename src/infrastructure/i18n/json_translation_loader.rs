use crate::domain::entities::language::Language;
use crate::domain::ports::secondary::translation_loader::TranslationLoader;
use std::collections::HashMap;

pub struct JsonTranslationLoader;

impl TranslationLoader for JsonTranslationLoader {
    fn load_translations(&self, language: &Language) -> HashMap<String, String> {
        let data = match language {
            Language::English => include_str!("../../../translations/en.json"),
            Language::French => include_str!("../../../translations/fr.json"),
        };
        serde_json::from_str(data).unwrap_or_default()
    }
}
