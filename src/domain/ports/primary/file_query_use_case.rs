use crate::domain::entities::pagination::PaginatedResult;
use crate::domain::errors::domain_error::DomainError;

#[async_trait::async_trait]
pub trait FileQueryUseCase: Send + Sync {
    async fn get_search_count(&self, query: &str) -> Result<i64, DomainError>;

    async fn search_files(
        &self,
        query: &str,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult, DomainError>;

    async fn list_files(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult, DomainError>;
}
