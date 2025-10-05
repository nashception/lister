use crate::domain::entities::file_entry::FileWithMetadata;
use crate::domain::errors::domain_error::DomainError;

pub trait FileQueryUseCase: Send + Sync {
    fn list_drive_names(&self) -> Result<Vec<String>, DomainError>;

    fn get_search_count(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<i64, DomainError>;

    fn search_files(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<FileWithMetadata>, DomainError>;
}
