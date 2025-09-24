use crate::domain::entities::file_entry::FileWithMetadata;
use crate::domain::errors::domain_error::DomainError;

#[async_trait::async_trait]
pub trait FileQueryUseCase: Send + Sync {
    async fn list_drive_names(&self) -> Result<Vec<String>, DomainError>;

    async fn get_search_count(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<i64, DomainError>;

    async fn search_files(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<FileWithMetadata>, DomainError>;
}
