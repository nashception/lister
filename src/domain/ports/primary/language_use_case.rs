use crate::domain::entities::language::Language;
use crate::domain::errors::domain_error::DomainError;
use std::collections::HashMap;

pub trait LanguageManagementUseCase: Send + Sync {
    /// Retrieves the currently selected application language.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while fetching the language from persistent storage.
    fn get_current_language(&self) -> Result<Language, DomainError>;

    /// Updates the current application language.
    ///
    /// Persists the provided [`Language`] to storage so it can be used
    /// for subsequent lookups and UI translations.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while saving the language setting.
    fn set_language(&self, language: Language) -> Result<(), DomainError>;

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
    fn load_translations(
        &self,
        language: &Language,
    ) -> Result<HashMap<String, String>, DomainError>;
}
