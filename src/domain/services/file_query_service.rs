use crate::domain::entities::file_entry::FileWithMetadata;
use crate::domain::errors::domain_error::DomainError;
use crate::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use crate::domain::ports::secondary::repositories::FileQueryRepository;
use std::sync::Arc;

pub struct FileQueryService {
    query_repo: Arc<dyn FileQueryRepository>,
}

impl FileQueryService {
    pub fn new(query_repo: Arc<dyn FileQueryRepository>) -> Self {
        Self { query_repo }
    }
}

impl FileQueryUseCase for FileQueryService {
    fn list_drive_names(&self) -> Result<Vec<String>, DomainError> {
        let drives = self.query_repo.find_all_drive_names()?;
        Ok(drives)
    }

    fn get_search_count(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<u64, DomainError> {
        let count = self
            .query_repo
            .count_search_results(selected_drive, query)?;
        Ok(count)
    }

    fn search_files(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<FileWithMetadata>, DomainError> {
        let offset = page * page_size;
        let limit = page_size;

        self.query_repo
            .search_files_paginated(selected_drive, query, offset, limit)
            .map_err(DomainError::Repository)
    }
}
