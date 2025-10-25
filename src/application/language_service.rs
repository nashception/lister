use crate::domain::entities::language::Language;
use crate::domain::errors::domain_error::DomainError;
use crate::infrastructure::database::language_repository::LanguageRepository;
use crate::infrastructure::i18n::json_translation_loader::JsonTranslationLoader;
use std::collections::HashMap;

pub struct LanguageService {
    language_repo: LanguageRepository,
    translation_loader: JsonTranslationLoader,
}

impl LanguageService {
    #[must_use]
    pub const fn new(
        language_repo: LanguageRepository,
        translation_loader: JsonTranslationLoader,
    ) -> Self {
        Self {
            language_repo,
            translation_loader,
        }
    }

    /// Retrieves the currently selected application language.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while fetching the language from persistent storage.
    pub fn get_current_language(&self) -> Result<Language, DomainError> {
        self.language_repo.get_language().map_err(DomainError::from)
    }

    /// Updates the current application language.
    ///
    /// Persists the provided [`Language`] to storage so it can be used
    /// for subsequent lookups and UI translations.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while saving the language setting.
    pub fn set_language(&self, language: &Language) -> Result<(), DomainError> {
        self.language_repo
            .set_language(language)
            .map_err(DomainError::from)
    }

    /// Loads all translations for the given language.
    ///
    /// Returns a [`HashMap`] containing key-value pairs representing
    /// localized strings for the specified language.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while fetching translations.
    /// - A [`DirectoryScannerError`](DomainError::DirectoryScannerError) occurs if reading translation files from disk fails.
    pub fn load_translations(
        &self,
        language: &Language,
    ) -> Result<HashMap<String, String>, DomainError> {
        Ok(self.translation_loader.load_translations(language))
    }
}
