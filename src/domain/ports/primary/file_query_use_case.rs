use crate::domain::entities::drive::Drive;
use crate::domain::entities::pagination::PaginatedResult;
use crate::domain::errors::domain_error::DomainError;

#[async_trait::async_trait]
pub trait FileQueryUseCase: Send + Sync {
    async fn list_drives(&self) -> Result<Vec<Drive>, DomainError>;

    async fn get_search_count(
        &self,
        selected_drive: &Option<Drive>,
        query: &Option<String>,
    ) -> Result<i64, DomainError>;

    async fn search_files(
        &self,
        selected_drive: &Option<Drive>,
        query: &Option<String>,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult, DomainError>;
}
