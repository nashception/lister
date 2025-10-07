use crate::domain::entities::file_entry::FileEntry;
use crate::domain::errors::domain_error::DomainError;
use std::path::Path;

pub trait FileIndexingUseCase: Send + Sync {
    /// Removes duplicate file entries for the given category and drive.
    ///
    /// Deletes all existing records in the database that match the specified
    /// category and drive combination.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while removing duplicates.
    fn remove_duplicates(&self, category: String, drive: String) -> Result<(), DomainError>;

    /// Scans the specified directory for files.
    ///
    /// Recursively walks the directory and collects metadata for each discovered file.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`DirectoryScannerError`](DomainError::DirectoryScannerError) occurs during file system traversal.
    fn scan_directory(&self, directory: &Path) -> Result<Vec<FileEntry>, DomainError>;

    /// Inserts scanned files into the database.
    ///
    /// Persists the given files under the specified category and drive, along with
    /// the remaining drive space. Returns the number of records inserted.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs during the insert operation.
    fn insert_in_database(
        &self,
        category: String,
        drive: String,
        drive_remaining_space: u64,
        files: Vec<FileEntry>,
    ) -> Result<usize, DomainError>;
}
