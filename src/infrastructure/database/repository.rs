use crate::domain::model::file_entry::{FileEntry, FileWithMetadata};
use crate::domain::model::language::Language;
use crate::infrastructure::database::binary_format::UuidSqlite;
use crate::infrastructure::database::conversion::{ToI64, ToU64};
use crate::infrastructure::database::entities::{
    FileWithMetadataDto, NewDriveEntryDto, NewFileCategoryDto, NewFileEntryDto,
};
use crate::infrastructure::database::pool::{InfrastructureError, SqliteRepositoryPool};
use crate::infrastructure::database::schema::{
    drive_entries, file_categories, file_entries, settings,
};
use crate::infrastructure::i18n::json_translation_loader::load_translations;
use crate::utils::dialogs::popup_error;
use chrono::Local;
use diesel::dsl::{exists, update};
use diesel::prelude::*;
use diesel::{QueryDsl, RunQueryDsl, SqliteConnection};
use rayon::prelude::*;
use std::collections::HashMap;

/// Repository for write operations on files, drives, and categories.
pub struct ListerRepository {
    pool: SqliteRepositoryPool,
}

impl ListerRepository {
    #[must_use]
    /// Creates a new [`ListerRepository`] with the given pool.
    pub const fn new(pool: SqliteRepositoryPool) -> Self {
        Self { pool }
    }

    /// Removes duplicate file entries for the specified category and drive.
    ///
    /// Deletes existing records in the database that match the given
    /// category and drive combination.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during the delete operation.
    pub fn remove_duplicates(
        &self,
        category: &str,
        drive: &str,
    ) -> Result<(), InfrastructureError> {
        self.pool.execute_in_transaction(|conn| {
            diesel::delete(
                file_entries::table.filter(exists(
                    drive_entries::table
                        .inner_join(file_categories::table)
                        .filter(drive_entries::id.eq(file_entries::drive_id))
                        .filter(file_categories::name.eq(category))
                        .filter(drive_entries::name.eq(drive)),
                )),
            )
            .execute(conn)?;

            Ok(())
        })
    }

    /// Saves a category, its drive, and associated files to the database.
    ///
    /// Inserts a new category and drive record, then stores the provided files
    /// under that drive.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during insert operations.
    pub fn save(
        &self,
        category: &str,
        drive: &str,
        drive_available_space: u64,
        files: &[FileEntry],
    ) -> Result<usize, InfrastructureError> {
        self.pool.execute_in_transaction(|conn| {
            let category_id = Self::save_category(category, conn)?;
            let drive_id = Self::save_drive(drive, drive_available_space, category_id, conn)?;
            Self::save_files(files, drive_id, conn)
        })
    }

    /// Deletes a drive, optionally filtered by category, from the database.
    ///
    /// If a category is provided, only the drive entries associated with that
    /// category will be removed. If no category is specified, all entries for
    /// the given drive will be deleted.
    ///
    /// # Parameters
    ///
    /// - `drive`: The name or identifier of the drive to delete.
    /// - `category`: An optional category name; if provided, deletion is limited
    ///   to drives under this category.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during the delete operation.
    pub fn delete(&self, drive: &str, category: Option<&str>) -> Result<(), InfrastructureError> {
        self.pool.execute_in_transaction(|conn| {
            if let Some(category_name) = category {
                let cat_ids = file_categories::table
                    .select(file_categories::id)
                    .filter(file_categories::name.eq(category_name));

                diesel::delete(
                    drive_entries::table
                        .filter(drive_entries::name.eq(drive))
                        .filter(drive_entries::category_id.eq_any(cat_ids)),
                )
                .execute(conn)?;
            } else {
                diesel::delete(drive_entries::table.filter(drive_entries::name.eq(drive)))
                    .execute(conn)?;
            }

            diesel::delete(file_categories::table.filter(
                file_categories::id.ne_all(drive_entries::table.select(drive_entries::category_id)),
            ))
            .execute(conn)?;

            Ok(())
        })
    }

    fn save_category(
        category: &str,
        conn: &mut SqliteConnection,
    ) -> Result<UuidSqlite, InfrastructureError> {
        let existing_id = file_categories::table
            .filter(file_categories::name.eq(category))
            .select(file_categories::id)
            .first::<UuidSqlite>(conn)
            .optional()?;

        if let Some(id) = existing_id {
            return Ok(id);
        }

        Ok(diesel::insert_into(file_categories::table)
            .values(NewFileCategoryDto {
                id: UuidSqlite::new(),
                name: category.to_string(),
            })
            .returning(file_categories::id)
            .get_result(conn)?)
    }

    fn save_drive(
        drive: &str,
        drive_available_space: u64,
        category_id: UuidSqlite,
        conn: &mut SqliteConnection,
    ) -> Result<UuidSqlite, InfrastructureError> {
        let existing_id = drive_entries::table
            .filter(
                drive_entries::name
                    .eq(drive)
                    .and(drive_entries::category_id.eq(&category_id)),
            )
            .select(drive_entries::id)
            .first::<UuidSqlite>(conn)
            .optional()?;

        if let Some(id) = existing_id {
            return Ok(id);
        }

        Self::update_same_drives_available_space(drive, drive_available_space, conn)?;

        Ok(diesel::insert_into(drive_entries::table)
            .values(NewDriveEntryDto {
                id: UuidSqlite::new(),
                category_id,
                name: drive.to_string(),
                available_space: drive_available_space.to_i64_or_zero(),
                insertion_time: Local::now().naive_local(),
            })
            .returning(drive_entries::id)
            .get_result(conn)?)
    }

    fn update_same_drives_available_space(
        drive: &str,
        drive_available_space: u64,
        conn: &mut SqliteConnection,
    ) -> Result<(), InfrastructureError> {
        update(drive_entries::table.filter(drive_entries::name.eq(drive)))
            .set(drive_entries::available_space.eq(drive_available_space.to_i64_or_zero()))
            .execute(conn)?;
        Ok(())
    }

    fn save_files(
        files: &[FileEntry],
        drive_id: UuidSqlite,
        conn: &mut SqliteConnection,
    ) -> Result<usize, InfrastructureError> {
        let dto_files: Vec<NewFileEntryDto> = files
            .par_iter()
            .map(|file_entry| (file_entry, drive_id).into())
            .collect();

        Ok(diesel::insert_into(file_entries::table)
            .values(&dto_files)
            .execute(conn)?)
    }

    fn search_pattern(query: &str) -> String {
        format!("%{query}%").replace(' ', "_")
    }

    /// Retrieves all used category names from the database based on a drive name.
    ///
    /// Returns a sorted list of unique used category names.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during query execution.
    pub fn find_all_category_names_for_drive(
        &self,
        drive: &str,
    ) -> Result<Vec<String>, InfrastructureError> {
        self.pool.execute_db_operation(|conn| {
            Ok(drive_entries::table
                .inner_join(file_categories::table)
                .filter(drive_entries::name.eq(drive))
                .select(file_categories::name)
                .distinct()
                .order(file_categories::name)
                .load::<String>(conn)?)
        })
    }

    /// Compacts the `SQLite` database file.
    ///
    /// This operation runs the `VACUUM` command, which rebuilds the database
    /// file to reclaim unused space and reduce fragmentation.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during query execution.
    pub fn compact(&self) -> Result<u64, InfrastructureError> {
        let size_before = std::fs::metadata("app.db")?.len();
        self.pool.execute_db_operation(|conn| {
            diesel::sql_query("VACUUM").execute(conn)?;
            diesel::sql_query("PRAGMA shrink_memory;").execute(conn)?;
            let size_after = std::fs::metadata("app.db")?.len();
            Ok(size_before - size_after)
        })
    }

    /// Retrieves all distinct drive names from the database.
    ///
    /// Returns a sorted list of unique drive names.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during query execution.
    pub fn find_all_drive_names(&self) -> Result<Vec<String>, InfrastructureError> {
        self.pool.execute_db_operation(|conn| {
            Ok(drive_entries::table
                .select(drive_entries::name)
                .distinct()
                .order(drive_entries::name)
                .load::<String>(conn)?)
        })
    }

    /// Counts the total number of files matching the provided search criteria.
    ///
    /// The search can be filtered by drive name and optional query pattern.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during query execution.
    pub fn count_search_results(
        &self,
        selected_drive: Option<&str>,
        query: Option<&str>,
    ) -> Result<u64, InfrastructureError> {
        let search_pattern = query.map(Self::search_pattern);

        self.pool.execute_db_operation(move |conn| {
            let mut query_builder = file_entries::table
                .inner_join(drive_entries::table)
                .into_boxed();

            if let Some(drive) = selected_drive {
                query_builder = query_builder.filter(drive_entries::name.eq(drive));
            }

            if let Some(pattern) = search_pattern {
                query_builder = query_builder.filter(file_entries::path.like(pattern));
            }

            Ok(query_builder
                .count()
                .get_result::<i64>(conn)?
                .to_u64_or_zero())
        })
    }

    /// Searches for files matching the given criteria with pagination support.
    ///
    /// Results can be filtered by drive and search query, and limited by
    /// offset and page size.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during query execution.
    pub fn search_files_paginated(
        &self,
        selected_drive: Option<&str>,
        query: Option<&str>,
        page: u64,
        page_size: u64,
    ) -> Result<Vec<FileWithMetadata>, InfrastructureError> {
        let offset = page * page_size;
        let limit = page_size;

        let search_pattern = query.map(Self::search_pattern);

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

            if let Some(drive) = selected_drive {
                query_builder = query_builder.filter(drive_entries::name.eq(drive));
            }

            if let Some(search) = search_pattern {
                query_builder = query_builder.filter(file_entries::path.like(search));
            }

            let entities = query_builder
                .limit(limit.to_i64_or_zero())
                .offset(offset.to_i64_or_zero())
                .load::<FileWithMetadataDto>(conn)?;

            Ok(entities
                .into_iter()
                .map(FileWithMetadataDto::into)
                .collect())
        })
    }

    #[must_use]
    pub fn translations(&self) -> (Language, HashMap<String, String>) {
        let current_language = self.get_language().unwrap_or_else(|error| {
            popup_error(&error);
            Language::default()
        });
        let translations = load_translations(&current_language).unwrap_or_else(|error| {
            popup_error(&error);
            HashMap::default()
        });

        (current_language, translations)
    }

    /// Retrieves the current application language from the database.
    ///
    /// Returns the stored language if present; otherwise defaults to [`Language::English`].
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during query execution.
    pub fn get_language(&self) -> Result<Language, InfrastructureError> {
        self.pool.execute_db_operation(|conn| {
            let lang: Option<String> = settings::table
                .filter(settings::key.eq("language"))
                .select(settings::value)
                .first(conn)
                .optional()?;

            Ok(lang.map_or_else(|| Language::English, |l| Language::new(&l)))
        })
    }

    /// Sets the application language in the database.
    ///
    /// Replaces any existing language setting with the provided value.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during the update operation.
    pub fn set_language(&self, language: &Language) -> Result<(), InfrastructureError> {
        self.pool.execute_db_operation(|conn| {
            diesel::replace_into(settings::table)
                .values((
                    settings::key.eq("language"),
                    settings::value.eq(language.code()),
                ))
                .execute(conn)?;
            Ok(())
        })
    }
}
