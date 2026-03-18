use crate::domain::model::file_entry::FileEntry;
use crate::infrastructure::database::binary_format::UuidSqlite;
use crate::infrastructure::database::conversion::ToI64;
use crate::infrastructure::database::entities::{
    NewDriveEntryDto, NewFileCategoryDto, NewFileEntryDto,
};
use crate::infrastructure::database::pool::{RepositoryError, SqliteRepositoryPool};
use crate::infrastructure::database::schema::{drive_entries, file_categories, file_entries};
use chrono::Local;
use diesel::dsl::{exists, update};
use diesel::prelude::*;
use diesel::{QueryDsl, RunQueryDsl, SqliteConnection};
use rayon::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

/// Repository for write operations on files, drives, and categories.
pub struct CommandRepository {
    pool: Arc<SqliteRepositoryPool>,
}

impl CommandRepository {
    #[must_use]
    /// Creates a new [`CommandRepository`] with the given pool.
    pub const fn new(pool: Arc<SqliteRepositoryPool>) -> Self {
        Self { pool }
    }

    /// Removes duplicate file entries for the specified category and drive.
    ///
    /// Deletes existing records in the database that match the given
    /// category and drive combination.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during the delete operation.
    pub fn remove_duplicates(&self, category: &str, drive: &str) -> Result<(), RepositoryError> {
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
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during insert operations.
    pub fn save(
        &self,
        category: &str,
        drive: &str,
        drive_available_space: u64,
        files: &[FileEntry],
    ) -> Result<usize, RepositoryError> {
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
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during the delete operation.
    pub fn delete(&self, drive: &str, category: Option<&str>) -> Result<(), RepositoryError> {
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
    ) -> Result<UuidSqlite, RepositoryError> {
        if let Ok(existing_id) = file_categories::table
            .filter(file_categories::name.eq(category))
            .select(file_categories::id)
            .first::<UuidSqlite>(conn)
        {
            return Ok(existing_id);
        }

        let category_id: UuidSqlite = diesel::insert_into(file_categories::table)
            .values(NewFileCategoryDto {
                id: UuidSqlite(Uuid::now_v7()),
                name: category.to_string(),
            })
            .returning(file_categories::id)
            .get_result(conn)?;
        Ok(category_id)
    }

    fn save_drive(
        drive: &str,
        drive_available_space: u64,
        category_id: UuidSqlite,
        conn: &mut SqliteConnection,
    ) -> Result<UuidSqlite, RepositoryError> {
        if let Ok(existing_id) = drive_entries::table
            .filter(
                drive_entries::name
                    .eq(drive)
                    .and(drive_entries::category_id.eq(&category_id)),
            )
            .select(drive_entries::id)
            .first::<UuidSqlite>(conn)
        {
            return Ok(existing_id);
        }

        let drive_id: UuidSqlite = diesel::insert_into(drive_entries::table)
            .values(NewDriveEntryDto {
                id: UuidSqlite(Uuid::now_v7()),
                category_id,
                name: drive.to_string(),
                available_space: drive_available_space.to_i64_or_zero(),
                insertion_time: Local::now().naive_local(),
            })
            .returning(drive_entries::id)
            .get_result(conn)?;

        Self::update_same_drives_available_space(drive, drive_available_space, conn)?;

        Ok(drive_id)
    }

    fn update_same_drives_available_space(
        drive: &str,
        drive_available_space: u64,
        conn: &mut SqliteConnection,
    ) -> Result<(), RepositoryError> {
        update(drive_entries::table.filter(drive_entries::name.eq(drive)))
            .set(drive_entries::available_space.eq(drive_available_space.to_i64_or_zero()))
            .execute(conn)?;
        Ok(())
    }

    fn save_files(
        files: &[FileEntry],
        drive_id: UuidSqlite,
        conn: &mut SqliteConnection,
    ) -> Result<usize, RepositoryError> {
        let dto_files: Vec<NewFileEntryDto> = files
            .par_iter()
            .map(|file_entry| (file_entry, drive_id).into())
            .collect();

        let insert_count = diesel::insert_into(file_entries::table)
            .values(&dto_files)
            .execute(conn)?;

        Ok(insert_count)
    }
}
