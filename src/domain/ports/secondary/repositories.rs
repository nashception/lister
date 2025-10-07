use crate::domain::entities::category::Category;
use crate::domain::entities::drive::{Drive, DriveToDelete};
use crate::domain::entities::file_entry::{FileEntry, FileWithMetadata};
use crate::domain::entities::language::Language;
use crate::domain::errors::repository_error::RepositoryError;

pub trait FileQueryRepository: Send + Sync {
    /// Retrieves all distinct drive names from the database.
    ///
    /// Returns a sorted list of unique drive names.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    fn find_all_drive_names(&self) -> Result<Vec<String>, RepositoryError>;

    /// Counts the total number of files matching the provided search criteria.
    ///
    /// The search can be filtered by drive name and optional query pattern.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    fn count_search_results(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<u64, RepositoryError>;

    /// Searches for files matching the given criteria with pagination support.
    ///
    /// Results can be filtered by drive and search query, and limited by
    /// offset and page size.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    fn search_files_paginated(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<FileWithMetadata>, RepositoryError>;
}

pub trait FileCommandRepository: Send + Sync {
    /// Removes duplicate file entries for the specified category and drive.
    ///
    /// Deletes existing records in the database that match the given
    /// category and drive combination.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during the delete operation.
    fn remove_duplicates(
        &self,
        category: Category,
        drive: DriveToDelete,
    ) -> Result<(), RepositoryError>;

    /// Saves a category, its drive, and associated files to the database.
    ///
    /// Inserts a new category and drive record, then stores the provided files
    /// under that drive.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during insert operations.
    fn save(
        &self,
        category: Category,
        drive: Drive,
        files: Vec<FileEntry>,
    ) -> Result<usize, RepositoryError>;
}

pub trait LanguageRepository: Send + Sync {
    /// Retrieves the current application language from the database.
    ///
    /// Returns the stored language if present; otherwise defaults to [`Language::English`].
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    fn get_language(&self) -> Result<Language, RepositoryError>;
    /// Sets the application language in the database.
    ///
    /// Replaces any existing language setting with the provided value.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during the update operation.
    fn set_language(&self, language: &Language) -> Result<(), RepositoryError>;
}
