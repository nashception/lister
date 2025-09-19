use crate::config::constants::{MIGRATIONS, TOKIO_RUNTIME};
use crate::domain::entities::category::Category;
use crate::domain::entities::drive::{Drive, DriveToDelete};
use crate::domain::entities::file_entry::{FileEntry, FileWithMetadata};
use crate::domain::entities::language::Language;
use crate::domain::entities::pagination::PaginatedResult;
use crate::domain::errors::repository_error::RepositoryError;
use crate::domain::ports::secondary::repositories::{
    FileCommandRepository, FileQueryRepository, LanguageRepository,
};
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
                available_space: drive.available_space,
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
            .set(drive_entries::available_space.eq(drive.available_space))
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
                weight: f.size_bytes,
            })
            .collect();

        let insert_count = diesel::insert_into(file_entries::table)
            .values(&dto_files)
            .execute(conn)?;

        Ok(insert_count)
    }

    fn count(
        selected_drive: &Option<String>,
        search_pattern: &Option<String>,
        conn: &mut DieselConnection,
    ) -> Result<i64, RepositoryError> {
        let mut query = file_entries::table
            .inner_join(drive_entries::table)
            .into_boxed();

        if let Some(drive) = selected_drive {
            query = query.filter(drive_entries::name.eq(drive));
        }

        if let Some(pattern) = search_pattern {
            query = query.filter(file_entries::path.like(pattern));
        }

        let count: i64 = query.count().get_result(conn)?;
        Ok(count)
    }

    fn search_pattern(query: &Option<String>) -> Option<String> {
        query
            .clone()
            .map(|dto| format!("%{}%", dto).replace(" ", "_"))
    }
}

#[async_trait::async_trait]
impl FileQueryRepository for SqliteFileRepository {
    async fn find_all_drive_names(&self) -> Result<Vec<String>, RepositoryError> {
        let pool = self.pool.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;

                let drives = drive_entries::table
                    .select(drive_entries::name)
                    .distinct()
                    .order(drive_entries::name)
                    .load::<String>(&mut conn)?;

                Ok(drives)
            })
            .await?
    }

    async fn search_files_paginated(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
        offset: i64,
        limit: i64,
    ) -> Result<PaginatedResult, RepositoryError> {
        let pool = self.pool.clone();
        let selected_drive = selected_drive.clone();
        let search_pattern = Self::search_pattern(query);
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;

                let total_count = Self::count(&selected_drive, &search_pattern, &mut conn)?;

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
                    .limit(limit)
                    .offset(offset)
                    .load::<FileWithMetadataDto>(&mut conn)?;

                let items = entities
                    .into_iter()
                    .map(|dto| FileWithMetadata {
                        category_name: dto.category_name,
                        drive_name: dto.drive_name,
                        drive_available_space: dto.drive_available_space,
                        drive_insertion_time: dto.drive_insertion_time,
                        path: dto.path,
                        size_bytes: dto.weight,
                    })
                    .collect();

                Ok(PaginatedResult { items, total_count })
            })
            .await?
    }

    async fn count_search_results(
        &self,
        selected_drive: &Option<String>,
        query: &Option<String>,
    ) -> Result<i64, RepositoryError> {
        let pool = self.pool.clone();
        let selected_drive = selected_drive.clone();
        let search_pattern = Self::search_pattern(query);
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;
                let count = Self::count(&selected_drive, &search_pattern, &mut conn)?;
                Ok(count)
            })
            .await?
    }
}

#[async_trait::async_trait]
impl FileCommandRepository for SqliteFileRepository {
    async fn remove_duplicates(
        &self,
        category: Category,
        drive: DriveToDelete,
    ) -> Result<(), RepositoryError> {
        let pool = self.pool.clone();

        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;
                conn.immediate_transaction::<_, RepositoryError, _>(|conn| {
                    Self::remove_duplicates(category.name, drive.name, conn)
                })
            })
            .await?
    }

    async fn save(
        &self,
        category: Category,
        drive: Drive,
        files: Vec<FileEntry>,
    ) -> Result<usize, RepositoryError> {
        let pool = self.pool.clone();
        let category_name = category.name;

        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;
                conn.immediate_transaction::<_, RepositoryError, _>(|conn| {
                    let category_id = Self::save_category(category_name, conn)?;

                    let drive_id = Self::save_drive(drive, category_id, conn)?;

                    Self::save_files(files, drive_id, conn)
                })
            })
            .await?
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

        Ok(lang
            .map(|l| Language::new(&l))
            .unwrap_or_else(Language::english))
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
