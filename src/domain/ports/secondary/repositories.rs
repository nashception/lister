use crate::domain::entities::category::Category;
use crate::domain::entities::drive::{Drive, DriveToDelete};
use crate::domain::entities::file_entry::{FileEntry, FileWithMetadata};
use crate::domain::entities::language::Language;
use crate::domain::errors::repository_error::RepositoryError;

#[async_trait::async_trait]
pub trait FileQueryRepository: Send + Sync {
    async fn find_all_drive_names(&self) -> Result<Vec<String>, RepositoryError>;

    async fn count_search_results(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<i64, RepositoryError>;

    async fn search_files_paginated(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<FileWithMetadata>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait FileCommandRepository: Send + Sync {
    async fn remove_duplicates(
        &self,
        category: Category,
        drive: DriveToDelete,
    ) -> Result<(), RepositoryError>;

    async fn save(
        &self,
        category: Category,
        drive: Drive,
        files: Vec<FileEntry>,
    ) -> Result<usize, RepositoryError>;
}

pub trait LanguageRepository: Send + Sync {
    fn get_language(&self) -> Result<Language, RepositoryError>;
    fn set_language(&self, language: &Language) -> Result<(), RepositoryError>;
}
