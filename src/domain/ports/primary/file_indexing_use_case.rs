use crate::domain::entities::file_entry::FileEntry;
use crate::domain::errors::domain_error::DomainError;
use std::path::PathBuf;

#[async_trait::async_trait]
pub trait FileIndexingUseCase: Send + Sync {
    async fn remove_duplicates(&self, category: String, drive: String) -> Result<(), DomainError>;
    async fn scan_directory(&self, directory: PathBuf) -> Result<Vec<FileEntry>, DomainError>;
    async fn insert_in_database(
        &self,
        category: String,
        drive: String,
        files: Vec<FileEntry>,
    ) -> Result<usize, DomainError>;
}
