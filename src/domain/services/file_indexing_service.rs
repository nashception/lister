use crate::domain::entities::category::Category;
use crate::domain::entities::drive::{Drive, DriveToDelete};
use crate::domain::entities::file_entry::FileEntry;
use crate::domain::errors::domain_error::DomainError;
use crate::domain::ports::primary::file_indexing_use_case::FileIndexingUseCase;
use crate::domain::ports::secondary::repositories::FileCommandRepository;
use crate::domain::services::directory_scanner;
use std::path::Path;
use std::sync::Arc;

pub struct FileIndexingService {
    command_repo: Arc<dyn FileCommandRepository>,
}

impl FileIndexingService {
    pub fn new(command_repo: Arc<dyn FileCommandRepository>) -> Self {
        Self { command_repo }
    }
}

impl FileIndexingUseCase for FileIndexingService {
    fn remove_duplicates(&self, category: String, drive: String) -> Result<(), DomainError> {
        self.command_repo
            .remove_duplicates(Category { name: category }, DriveToDelete { name: drive })?;
        Ok(())
    }

    fn scan_directory(&self, directory: &Path) -> Result<Vec<FileEntry>, DomainError> {
        let files = directory_scanner::scan_directory(directory)?;
        Ok(files)
    }

    fn insert_in_database(
        &self,
        category: String,
        drive: String,
        drive_available_space: u64,
        files: Vec<FileEntry>,
    ) -> Result<usize, DomainError> {
        let files_count = self.command_repo.save(
            Category { name: category },
            Drive {
                name: drive,
                available_space: drive_available_space,
            },
            files,
        )?;
        Ok(files_count)
    }
}
