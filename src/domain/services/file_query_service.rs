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

#[async_trait::async_trait]
impl FileQueryUseCase for FileQueryService {
    async fn list_drive_names(&self) -> Result<Vec<String>, DomainError> {
        let drives = self.query_repo.find_all_drive_names().await?;
        Ok(drives)
    }

    async fn get_search_count(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<i64, DomainError> {
        let count = self
            .query_repo
            .count_search_results(selected_drive, query)
            .await?;
        Ok(count)
    }

    async fn search_files(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<FileWithMetadata>, DomainError> {
        let offset = (page * page_size) as i64;
        let limit = page_size as i64;

        self.query_repo
            .search_files_paginated(selected_drive, query, offset, limit)
            .await
            .map_err(DomainError::Repository)
    }
}
