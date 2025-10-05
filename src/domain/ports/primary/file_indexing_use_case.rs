use crate::domain::entities::file_entry::FileEntry;
use crate::domain::errors::domain_error::DomainError;
use std::path::Path;

pub trait FileIndexingUseCase: Send + Sync {
    fn remove_duplicates(&self, category: String, drive: String) -> Result<(), DomainError>;

    fn scan_directory(&self, directory: &Path) -> Result<Vec<FileEntry>, DomainError>;

    fn insert_in_database(
        &self,
        category: String,
        drive: String,
        drive_remaining_space: i64,
        files: Vec<FileEntry>,
    ) -> Result<usize, DomainError>;
}
