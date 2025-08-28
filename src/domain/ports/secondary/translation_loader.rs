use crate::domain::entities::language::Language;
use std::collections::HashMap;

pub trait TranslationLoader: Send + Sync {
    fn load_translations(&self, language: &Language) -> HashMap<String, String>;
}
