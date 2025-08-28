use crate::domain::entities::language::Language;
use crate::domain::errors::domain_error::DomainError;
use crate::domain::ports::primary::language_use_case::LanguageManagementUseCase;
use crate::domain::ports::secondary::repositories::LanguageRepository;
use crate::domain::ports::secondary::translation_loader::TranslationLoader;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LanguageService {
    language_repo: Arc<dyn LanguageRepository>,
    translation_loader: Arc<dyn TranslationLoader>,
}

impl LanguageService {
    pub fn new(
        language_repo: Arc<dyn LanguageRepository>,
        translation_loader: Arc<dyn TranslationLoader>,
    ) -> Self {
        Self {
            language_repo,
            translation_loader,
        }
    }
}

impl LanguageManagementUseCase for LanguageService {
    fn get_current_language(&self) -> Result<Language, DomainError> {
        self.language_repo
            .get_language()
            .map_err(DomainError::Repository)
    }

    fn set_language(&self, language: Language) -> Result<(), DomainError> {
        self.language_repo
            .set_language(&language)
            .map_err(DomainError::Repository)
    }

    fn load_translations(
        &self,
        language: &Language,
    ) -> Result<HashMap<String, String>, DomainError> {
        Ok(self.translation_loader.load_translations(language))
    }
}
