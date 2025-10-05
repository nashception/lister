use crate::domain::entities::file_entry::FileWithMetadata;
use crate::domain::errors::domain_error::DomainError;

pub trait FileQueryUseCase: Send + Sync {
    /// Retrieves all available drive names.
    ///
    /// Returns a list of distinct drive names accessible in the system.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while fetching drive names from storage.
    fn list_drive_names(&self) -> Result<Vec<String>, DomainError>;

    /// Counts the total number of files matching the given search criteria.
    ///
    /// The count can be filtered by selected drive and optional query pattern.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while executing the count query.
    fn get_search_count(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<u64, DomainError>;

    /// Searches for files matching the given criteria with pagination.
    ///
    /// Returns a subset of matching files based on the provided page and page size.
    /// The search can be filtered by drive and query string.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while executing the search query.
    fn search_files(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<FileWithMetadata>, DomainError>;
}
