use crate::config::constants::MIGRATIONS;
use crate::domain::entities::category::Category;
use crate::domain::entities::drive::{Drive, DriveToDelete};
use crate::domain::entities::file_entry::{FileEntry, FileWithMetadata};
use crate::domain::entities::language::Language;
use crate::domain::errors::repository_error::RepositoryError;
use crate::domain::ports::secondary::repositories::{
    FileCommandRepository, FileQueryRepository, LanguageRepository,
};
use crate::infrastructure::database::conversion::{ToI64, ToU64};
use crate::infrastructure::database::entities::{
    FileWithMetadataDto, NewDriveEntryDto, NewFileCategoryDto, NewFileEntryDto,
};
use crate::infrastructure::database::schema::{
    drive_entries, file_categories, file_entries, settings,
};
use chrono::Local;
use diesel::dsl::{exists, update};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection, TextExpressionMethods};
use diesel_migrations::MigrationHarness;

type DieselPool = Pool<ConnectionManager<SqliteConnection>>;
type DieselConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

pub struct SqliteFileRepository {
    pool: DieselPool,
}

impl SqliteFileRepository {
    /// Creates a new [`SqliteFileRepository`] using the given database URL.
    ///
    /// Initializes the connection pool, enables foreign keys,
    /// and runs all pending migrations.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while creating the connection pool.
    /// - A [`Database`](RepositoryError::Database) error occurs while enabling foreign keys.
    /// - A [`Migration`](RepositoryError::Migration) error occurs while running migrations.
    pub fn new(database_url: &str) -> Result<Self, RepositoryError> {
        let pool = Self::create_pool(database_url)?;
        Self::enable_foreign_keys(&pool)?;
        Self::run_migrations(&pool)?;
        Ok(Self { pool })
    }

    fn create_pool(database_url: &str) -> Result<DieselPool, RepositoryError> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        Pool::builder()
            .build(manager)
            .map_err(RepositoryError::ConnectionPool)
    }

    fn enable_foreign_keys(pool: &DieselPool) -> Result<(), RepositoryError> {
        let mut conn = pool.get().map_err(RepositoryError::ConnectionPool)?;
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut conn)
            .map_err(RepositoryError::Database)?;
        Ok(())
    }

    fn run_migrations(pool: &DieselPool) -> Result<(), RepositoryError> {
        let mut conn = pool.get().map_err(RepositoryError::ConnectionPool)?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|err| RepositoryError::Migration(err.to_string()))?;
        Ok(())
    }

    fn execute_db_operation<F, R>(&self, operation: F) -> Result<R, RepositoryError>
    where
        F: FnOnce(&mut DieselConnection) -> Result<R, RepositoryError> + Send + 'static,
        R: Send + 'static,
    {
        let pool = self.pool.clone();
        let mut conn = pool.get().map_err(RepositoryError::ConnectionPool)?;
        operation(&mut conn)
    }

    fn remove_duplicates(
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
    ) -> Result<i32, RepositoryError> {
        let category_id = diesel::insert_into(file_categories::table)
            .values(NewFileCategoryDto {
                name: category_name,
            })
            .returning(file_categories::id)
            .get_result(conn)?;
        Ok(category_id)
    }

    fn save_drive(
        drive: Drive,
        category_id: i32,
        conn: &mut SqliteConnection,
    ) -> Result<i32, RepositoryError> {
        let drive_id = diesel::insert_into(drive_entries::table)
            .values(NewDriveEntryDto {
                category_id,
                name: drive.name.clone(),
                available_space: drive.available_space.to_i64_or_zero(),
                insertion_time: Local::now().naive_local(),
            })
            .returning(drive_entries::id)
            .get_result(conn)?;

        Self::update_same_drives_available_space(drive, conn)?;

        Ok(drive_id)
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
        drive_id: i32,
        conn: &mut SqliteConnection,
    ) -> Result<usize, RepositoryError> {
        let dto_files: Vec<NewFileEntryDto> = files
            .into_iter()
            .map(|f| NewFileEntryDto {
                drive_id,
                path: f.path,
                weight: f.size_bytes.to_i64_or_zero(),
            })
            .collect();

        let insert_count = diesel::insert_into(file_entries::table)
            .values(&dto_files)
            .execute(conn)?;

        Ok(insert_count)
    }

    fn search_pattern(query: &String) -> String {
        format!("%{query}%").replace(' ', "_")
    }
}

impl FileQueryRepository for SqliteFileRepository {
    fn find_all_drive_names(&self) -> Result<Vec<String>, RepositoryError> {
        self.execute_db_operation(|conn| {
            let drives = drive_entries::table
                .select(drive_entries::name)
                .distinct()
                .order(drive_entries::name)
                .load::<String>(conn)?;
            Ok(drives)
        })
    }

    fn count_search_results(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<u64, RepositoryError> {
        let selected_drive = selected_drive.clone();
        let search_pattern = query.as_ref().map(Self::search_pattern);

        self.execute_db_operation(move |conn| {
            let mut query1 = file_entries::table
                .inner_join(drive_entries::table)
                .into_boxed();

            if let Some(drive) = &selected_drive {
                query1 = query1.filter(drive_entries::name.eq(drive));
            }

            if let Some(pattern) = &search_pattern {
                query1 = query1.filter(file_entries::path.like(pattern));
            }

            let count: i64 = query1.count().get_result(conn)?;
            Ok(count.to_u64_or_zero())
        })
    }

    fn search_files_paginated(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<FileWithMetadata>, RepositoryError> {
        let selected_drive = selected_drive.clone();
        let search_pattern = query.as_ref().map(Self::search_pattern);

        self.execute_db_operation(move |conn| {
            let mut query = file_entries::table
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

            // Conditionally add drive filter
            if let Some(drive) = &selected_drive {
                query = query.filter(drive_entries::name.eq(drive));
            }

            // Apply search pattern filter
            if let Some(search) = &search_pattern {
                query = query.filter(file_entries::path.like(search));
            }

            let entities = query
                .limit(limit.to_i64_or_zero())
                .offset(offset.to_i64_or_zero())
                .load::<FileWithMetadataDto>(conn)?;

            let items = entities
                .into_iter()
                .map(|dto| FileWithMetadata {
                    category_name: dto.category_name,
                    drive_name: dto.drive_name,
                    drive_available_space: dto.drive_available_space.to_u64_or_zero(),
                    drive_insertion_time: dto.drive_insertion_time,
                    path: dto.path,
                    size_bytes: dto.weight.to_u64_or_zero(),
                })
                .collect();

            Ok(items)
        })
    }
}

impl FileCommandRepository for SqliteFileRepository {
    fn remove_duplicates(
        &self,
        category: Category,
        drive: DriveToDelete,
    ) -> Result<(), RepositoryError> {
        self.execute_db_operation(move |conn| {
            conn.immediate_transaction::<_, RepositoryError, _>(|conn| {
                Self::remove_duplicates(category.name, drive.name, conn)
            })
        })
    }

    fn save(
        &self,
        category: Category,
        drive: Drive,
        files: Vec<FileEntry>,
    ) -> Result<usize, RepositoryError> {
        let category_name = category.name;

        self.execute_db_operation(move |conn| {
            conn.immediate_transaction::<_, RepositoryError, _>(|conn| {
                let category_id = Self::save_category(category_name, conn)?;
                let drive_id = Self::save_drive(drive, category_id, conn)?;
                Self::save_files(files, drive_id, conn)
            })
        })
    }
}

impl LanguageRepository for SqliteFileRepository {
    fn get_language(&self) -> Result<Language, RepositoryError> {
        let mut conn = self.pool.get()?;
        let lang: Option<String> = settings::table
            .filter(settings::key.eq("language"))
            .select(settings::value)
            .first(&mut conn)
            .optional()?;

        Ok(lang.map_or_else(|| Language::English, |l| Language::new(&l)))
    }

    fn set_language(&self, language: &Language) -> Result<(), RepositoryError> {
        let mut conn = self.pool.get()?;
        diesel::replace_into(settings::table)
            .values((
                settings::key.eq("language"),
                settings::value.eq(language.code()),
            ))
            .execute(&mut conn)?;
        Ok(())
    }
}
