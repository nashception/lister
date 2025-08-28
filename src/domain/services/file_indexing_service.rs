use crate::domain::entities::category::Category;
use crate::domain::entities::drive::Drive;
use crate::domain::entities::file_entry::FileEntry;
use crate::domain::errors::domain_error::DomainError;
use crate::domain::ports::primary::file_indexing_use_case::FileIndexingUseCase;
use crate::domain::ports::secondary::repositories::FileCommandRepository;
use crate::domain::services::directory_scanner::DirectoryScanner;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FileIndexingService {
    command_repo: Arc<dyn FileCommandRepository>,
}

impl FileIndexingService {
    pub fn new(command_repo: Arc<dyn FileCommandRepository>) -> Self {
        Self { command_repo }
    }
}

#[async_trait::async_trait]
impl FileIndexingUseCase for FileIndexingService {
    async fn remove_duplicates(&self, category: String, drive: String) -> Result<(), DomainError> {
        let files_count = self
            .command_repo
            .remove_duplicates(Category { name: category }, Drive { name: drive })
            .await?;
        Ok(files_count)
    }

    async fn scan_directory(&self, directory: PathBuf) -> Result<Vec<FileEntry>, DomainError> {
        let scanner = DirectoryScanner::new(directory);
        let files = scanner.scan_directory().await?;
        Ok(files)
    }

    async fn insert_in_database(
        &self,
        category: String,
        drive: String,
        files: Vec<FileEntry>,
    ) -> Result<usize, DomainError> {
        let files_count = self
            .command_repo
            .save(Category { name: category }, Drive { name: drive }, files)
            .await?;
        Ok(files_count)
    }
}
