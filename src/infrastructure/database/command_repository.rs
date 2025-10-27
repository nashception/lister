use crate::domain::entities::category::Category;
use crate::domain::entities::drive::{Drive, DriveToDelete};
use crate::domain::entities::file_entry::FileEntry;
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
    pub fn remove_duplicates(
        &self,
        category: Category,
        drive: DriveToDelete,
    ) -> Result<(), RepositoryError> {
        let category_name = category.name;
        let drive_name = drive.name;

        self.pool.execute_in_transaction(move |conn| {
            Self::do_remove_duplicates(category_name, drive_name, conn)
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
        category: Category,
        drive: Drive,
        files: Vec<FileEntry>,
    ) -> Result<usize, RepositoryError> {
        let category_name = category.name;

        self.pool.execute_in_transaction(move |conn| {
            let category_id = Self::save_category(category_name, conn)?;
            let drive_id = Self::save_drive(drive, category_id, conn)?;
            Self::save_files(files, drive_id, conn)
        })
    }

    fn do_remove_duplicates(
        category_name: String,
        drive_name: String,
        conn: &mut SqliteConnection,
    ) -> Result<(), RepositoryError> {
        diesel::delete(
            file_entries::table.filter(exists(
                drive_entries::table
                    .inner_join(file_categories::table)
                    .filter(drive_entries::id.eq(file_entries::drive_id))
                    .filter(file_categories::name.eq(category_name))
                    .filter(drive_entries::name.eq(drive_name)),
            )),
        )
        .execute(conn)?;

        Ok(())
    }

    fn save_category(
        category_name: String,
        conn: &mut SqliteConnection,
    ) -> Result<Uuid, RepositoryError> {
        if let Ok(existing_id) = file_categories::table
            .filter(file_categories::name.eq(&category_name))
            .select(file_categories::id)
            .first::<String>(conn)
        {
            return Ok(Uuid::parse_str(&existing_id).unwrap());
        }

        let category_id: String = diesel::insert_into(file_categories::table)
            .values(NewFileCategoryDto {
                id: Uuid::new_v4().to_string(),
                name: category_name,
            })
            .returning(file_categories::id)
            .get_result(conn)?;
        Ok(Uuid::parse_str(&category_id).unwrap())
    }

    fn save_drive(
        drive: Drive,
        category_id: Uuid,
        conn: &mut SqliteConnection,
    ) -> Result<Uuid, RepositoryError> {
        let category_id_string = category_id.to_string();
        if let Ok(existing_id) = drive_entries::table
            .filter(
                drive_entries::name
                    .eq(&drive.name)
                    .and(drive_entries::category_id.eq(&category_id_string)),
            )
            .select(drive_entries::id)
            .first::<String>(conn)
        {
            return Ok(Uuid::parse_str(&existing_id).unwrap());
        }

        let drive_id: String = diesel::insert_into(drive_entries::table)
            .values(NewDriveEntryDto {
                id: Uuid::new_v4().to_string(),
                category_id: category_id_string,
                name: drive.name.clone(),
                available_space: drive.available_space.to_i64_or_zero(),
                insertion_time: Local::now().naive_local(),
            })
            .returning(drive_entries::id)
            .get_result(conn)?;

        Self::update_same_drives_available_space(drive, conn)?;

        Ok(Uuid::parse_str(&drive_id).unwrap())
    }

    fn update_same_drives_available_space(
        drive: Drive,
        conn: &mut SqliteConnection,
    ) -> Result<(), RepositoryError> {
        update(drive_entries::table.filter(drive_entries::name.eq(drive.name)))
            .set(drive_entries::available_space.eq(drive.available_space.to_i64_or_zero()))
            .execute(conn)?;
        Ok(())
    }

    fn save_files(
        files: Vec<FileEntry>,
        drive_id: Uuid,
        conn: &mut SqliteConnection,
    ) -> Result<usize, RepositoryError> {
        let dto_files: Vec<NewFileEntryDto> = files
            .into_par_iter()
            .map(|f| NewFileEntryDto {
                id: Uuid::new_v4().to_string(),
                drive_id: drive_id.to_string(),
                path: f.path,
                weight: f.size_bytes.to_i64_or_zero(),
            })
            .collect();

        let insert_count = diesel::insert_into(file_entries::table)
            .values(&dto_files)
            .execute(conn)?;

        Ok(insert_count)
    }
}
