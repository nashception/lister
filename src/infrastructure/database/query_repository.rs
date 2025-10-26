use crate::domain::entities::file_entry::FileWithMetadata;
use crate::infrastructure::database::conversion::{ToI64, ToU64};
use crate::infrastructure::database::entities::FileWithMetadataDto;
use crate::infrastructure::database::pool::{RepositoryError, SqliteRepositoryPool};
use crate::infrastructure::database::schema::{drive_entries, file_categories, file_entries};
use diesel::prelude::*;
use diesel::{QueryDsl, RunQueryDsl, TextExpressionMethods};
use std::sync::Arc;

/// Repository for read-only file and drive queries.
pub struct QueryRepository {
    pool: Arc<SqliteRepositoryPool>,
}

impl QueryRepository {
    /// Creates a new [`QueryRepository`] with the given pool.
    #[must_use]
    pub const fn new(pool: Arc<SqliteRepositoryPool>) -> Self {
        Self { pool }
    }

    /// Retrieves all distinct drive names from the database.
    ///
    /// Returns a sorted list of unique drive names.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    pub fn find_all_drive_names(&self) -> Result<Vec<String>, RepositoryError> {
        self.pool.execute_db_operation(|conn| {
            let drives = drive_entries::table
                .select(drive_entries::name)
                .distinct()
                .order(drive_entries::name)
                .load::<String>(conn)?;
            Ok(drives)
        })
    }

    /// Counts the total number of files matching the provided search criteria.
    ///
    /// The search can be filtered by drive name and optional query pattern.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    pub fn count_search_results(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<u64, RepositoryError> {
        let selected_drive = selected_drive.clone();
        let search_pattern = query.as_ref().map(Self::search_pattern);

        self.pool.execute_db_operation(move |conn| {
            let mut query_builder = file_entries::table
                .inner_join(drive_entries::table)
                .into_boxed();

            if let Some(drive) = &selected_drive {
                query_builder = query_builder.filter(drive_entries::name.eq(drive));
            }

            if let Some(pattern) = &search_pattern {
                query_builder = query_builder.filter(file_entries::path.like(pattern));
            }

            let count: i64 = query_builder.count().get_result(conn)?;
            Ok(count.to_u64_or_zero())
        })
    }

    /// Searches for files matching the given criteria with pagination support.
    ///
    /// Results can be filtered by drive and search query, and limited by
    /// offset and page size.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    pub fn search_files_paginated(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<FileWithMetadata>, RepositoryError> {
        let selected_drive = selected_drive.clone();
        let search_pattern = query.as_ref().map(Self::search_pattern);

        self.pool.execute_db_operation(move |conn| {
            let mut query_builder = file_entries::table
                .inner_join(drive_entries::table.inner_join(file_categories::table))
                .select((
                    file_categories::name,
                    drive_entries::name,
                    drive_entries::available_space,
                    drive_entries::insertion_time,
                    file_entries::path,
                    file_entries::weight,
                ))
                .into_boxed();

            if let Some(drive) = &selected_drive {
                query_builder = query_builder.filter(drive_entries::name.eq(drive));
            }

            if let Some(search) = &search_pattern {
                query_builder = query_builder.filter(file_entries::path.like(search));
            }

            let entities = query_builder
                .limit(limit.to_i64_or_zero())
                .offset(offset.to_i64_or_zero())
                .load::<FileWithMetadataDto>(conn)?;

            let items = entities
                .into_iter()
                .map(FileWithMetadataDto::into)
                .collect();

            Ok(items)
        })
    }

    fn search_pattern(query: &String) -> String {
        format!("%{query}%").replace(' ', "_")
    }
}