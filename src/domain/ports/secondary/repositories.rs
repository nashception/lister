use crate::domain::entities::category::Category;
use crate::domain::entities::drive::{Drive, DriveToDelete};
use crate::domain::entities::file_entry::{FileEntry, FileWithMetadata};
use crate::domain::entities::language::Language;
use crate::domain::errors::repository_error::RepositoryError;

pub trait FileQueryRepository: Send + Sync {
    fn find_all_drive_names(&self) -> Result<Vec<String>, RepositoryError>;

    fn count_search_results(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<i64, RepositoryError>;

    fn search_files_paginated(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<FileWithMetadata>, RepositoryError>;
}

pub trait FileCommandRepository: Send + Sync {
    fn remove_duplicates(
        &self,
        category: Category,
        drive: DriveToDelete,
    ) -> Result<(), RepositoryError>;

    fn save(
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
