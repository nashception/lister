use crate::domain::errors::domain_error::DomainError;
use crate::domain::model::file_entry::FileWithMetadata;
use crate::infrastructure::database::query_repository::QueryRepository;

pub struct FileQueryService {
    query_repo: QueryRepository,
}

impl FileQueryService {
    #[must_use]
    pub const fn new(query_repo: QueryRepository) -> Self {
        Self { query_repo }
    }

    /// Retrieves all available drive names.
    ///
    /// Returns a list of distinct drive names accessible in the system.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::RepositoryFailure) error occurs while fetching drive names from storage.
    pub fn list_drive_names(&self) -> Result<Vec<String>, DomainError> {
        let drives = self.query_repo.find_all_drive_names()?;
        Ok(drives)
    }

    /// Counts the total number of files matching the given search criteria.
    ///
    /// The count can be filtered by selected drive and optional query pattern.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::RepositoryFailure) error occurs while executing the count query.
    pub fn get_search_count(
        &self,
        selected_drive: Option<&str>,
        query: Option<&str>,
    ) -> Result<u64, DomainError> {
        let count = self
            .query_repo
            .count_search_results(selected_drive, query)?;
        Ok(count)
    }

    /// Searches for files matching the given criteria with pagination.
    ///
    /// Returns a subset of matching files based on the provided page and page size.
    /// The search can be filtered by drive and query string.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::RepositoryFailure) error occurs while executing the search query.
    pub fn search_files(
        &self,
        selected_drive: Option<&str>,
        query: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<FileWithMetadata>, DomainError> {
        let offset = page * page_size;
        let limit = page_size;

        self.query_repo
            .search_files_paginated(selected_drive, query, offset, limit)
            .map_err(DomainError::from)
    }

    /// Retrieves all used category names based on a drive name.
    ///
    /// Returns a list of distinct category names accessible in the system.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::RepositoryFailure) error occurs while fetching drive names from storage.
    pub fn list_category_names_for_drive(&self, drive: &str) -> Result<Vec<String>, DomainError> {
        let drives = self.query_repo.find_all_category_names_for_drive(drive)?;
        Ok(drives)
    }

    /// Compacts the database and returns the reclaimed disk space.
    ///
    /// This operation runs a database compaction which rebuilds the database
    /// file and removes unused space. The returned value represents the number
    /// of bytes reclaimed after the operation.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::RepositoryFailure) error occurs while executing the compaction.
    /// - An [`Io`](DomainError::IoError) error occurs while reading the database file metadata.
    pub fn compact(&self) -> Result<u64, DomainError> {
        let size_before = std::fs::metadata("app.db")?.len();
        self.query_repo.compact()?;
        let size_after = std::fs::metadata("app.db")?.len();
        Ok(size_before - size_after)
    }
}
