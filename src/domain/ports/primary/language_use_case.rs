use crate::domain::entities::language::Language;
use crate::domain::errors::domain_error::DomainError;
use std::collections::HashMap;

pub trait LanguageManagementUseCase: Send + Sync {
    fn get_current_language(&self) -> Result<Language, DomainError>;

    fn set_language(&self, language: Language) -> Result<(), DomainError>;

    fn load_translations(
        &self,
        language: &Language,
    ) -> Result<HashMap<String, String>, DomainError>;
}
