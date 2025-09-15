use crate::config::constants::CACHED_SIZE;
use crate::domain::entities::drive::Drive;
use crate::domain::entities::pagination::PaginatedResult;
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
    async fn list_drives(&self) -> Result<Vec<Drive>, DomainError> {
        let drives = self
            .query_repo
            .find_all_drives()
            .await?;
        Ok(drives)
    }

    async fn get_search_count(&self, query: &str) -> Result<i64, DomainError> {
        let count = self.query_repo.count_search_results(query).await?;
        Ok(count)
    }

    async fn search_files(
        &self,
        query: &str,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult, DomainError> {
        let offset = (page * page_size) as i64;
        let limit = page_size as i64;

        // Optimize small result sets by caching
        let count = self.query_repo.count_search_results(query).await?;
        if count <= CACHED_SIZE {
            self.query_repo
                .search_files_paginated(query, 0, count)
                .await
                .map(|mut result| {
                    let start = offset as usize;
                    let end = (start + page_size).min(result.items.len());
                    result.items = if start < result.items.len() {
                        result.items[start..end].to_vec()
                    } else {
                        Vec::new()
                    };
                    result
                })
                .map_err(DomainError::Repository)
        } else {
            self.query_repo
                .search_files_paginated(query, offset, limit)
                .await
                .map_err(DomainError::Repository)
        }
    }

    async fn list_files(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult, DomainError> {
        let offset = (page * page_size) as i64;
        let limit = page_size as i64;
        self.query_repo
            .find_files_paginated(offset, limit)
            .await
            .map_err(DomainError::Repository)
    }
}
