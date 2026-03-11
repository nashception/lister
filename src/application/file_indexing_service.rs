use crate::application::directory_scanner;
use crate::domain::errors::domain_error::DomainError;
use crate::domain::model::file_entry::FileEntry;
use crate::infrastructure::database::command_repository::CommandRepository;
use std::path::Path;
use std::sync::Arc;

pub struct FileIndexingService {
    command_repo: Arc<CommandRepository>,
}

impl FileIndexingService {
    #[must_use]
    pub const fn new(command_repo: Arc<CommandRepository>) -> Self {
        Self { command_repo }
    }

    /// Removes duplicate file entries for the given category and drive.
    ///
    /// Deletes all existing records in the database that match the specified
    /// category and drive combination.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while removing duplicates.
    pub fn remove_duplicates(&self, category: &str, drive: &str) -> Result<(), DomainError> {
        self.command_repo.remove_duplicates(category, drive)?;
        Ok(())
    }

    /// Scans the specified directory for files.
    ///
    /// Recursively walks the directory and collects metadata for each discovered file.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`DirectoryScannerError`](DomainError::DirectoryScannerError) occurs during file system traversal.
    pub fn scan_directory(&self, directory: &Path) -> Result<Vec<FileEntry>, DomainError> {
        let files = directory_scanner::scan_directory(directory)?;
        Ok(files)
    }

    /// Inserts scanned files into the database.
    ///
    /// Persists the given files under the specified category and drive, along with
    /// the remaining drive space. Returns the number of records inserted.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs during the insert operation.
    pub fn insert_in_database(
        &self,
        category: &str,
        drive: &str,
        drive_available_space: u64,
        files: &[FileEntry],
    ) -> Result<usize, DomainError> {
        let files_count = self
            .command_repo
            .save(category, drive, drive_available_space, files)?;
        Ok(files_count)
    }
}
