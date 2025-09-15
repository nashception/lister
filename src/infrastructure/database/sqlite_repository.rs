use crate::config::constants::{MIGRATIONS, TOKIO_RUNTIME};
use crate::domain::entities::category::Category;
use crate::domain::entities::drive::Drive;
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
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection, TextExpressionMethods};
use diesel_migrations::MigrationHarness;
use crate::infrastructure::database::schema::drive_entries::dsl;

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
        drive_name: String,
        category_id: i32,
        conn: &mut SqliteConnection,
    ) -> Result<i32, RepositoryError> {
        let drive_id = diesel::insert_into(drive_entries::table)
            .values(NewDriveEntryDto {
                category_id,
                name: drive_name,
            })
            .returning(drive_entries::id)
            .get_result(conn)?;
        Ok(drive_id)
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
}

#[async_trait::async_trait]
impl FileQueryRepository for SqliteFileRepository {
    async fn find_all_drives(&self) -> Result<Vec<Drive>, RepositoryError> {
        let pool = self.pool.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;

                let drives = dsl::drive_entries
                    .select(drive_entries::name)
                    .order(drive_entries::name)
                    .load::<String>(&mut conn)?;

                Ok(drives.into_iter().map(|name| Drive { name }).collect())
            })
            .await?
    }

    async fn find_files_paginated(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<PaginatedResult, RepositoryError> {
        let pool = self.pool.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;

                let total_count: i64 = file_entries::table.count().get_result(&mut conn)?;

                let entities = file_entries::table
                    .inner_join(drive_entries::table.inner_join(file_categories::table))
                    .select((
                        file_categories::name,
                        drive_entries::name,
                        file_entries::path,
                        file_entries::weight,
                    ))
                    .limit(limit)
                    .offset(offset)
                    .load::<FileWithMetadataDto>(&mut conn)?;

                let items = entities
                    .into_iter()
                    .map(|dto| FileWithMetadata {
                        category_name: dto.category_name,
                        drive_name: dto.drive_name,
                        path: dto.path,
                        size_bytes: dto.weight,
                    })
                    .collect();

                Ok(PaginatedResult { items, total_count })
            })
            .await?
    }

    async fn search_files_paginated(
        &self,
        selected_drive: &Option<Drive>,
        query: &str,
        offset: i64,
        limit: i64,
    ) -> Result<PaginatedResult, RepositoryError> {
        let pool = self.pool.clone();
        let selected_drive = selected_drive.clone();
        let search_pattern = format!("%{}%", query.replace(" ", "_"));
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;

                let total_count = count(&selected_drive, &search_pattern, &mut conn)?;

                let entities = file_entries::table
                    .inner_join(drive_entries::table.inner_join(file_categories::table))
                    .select((
                        file_categories::name,
                        drive_entries::name,
                        file_entries::path,
                        file_entries::weight,
                    ))
                    .filter(file_entries::path.like(&search_pattern))
                    .limit(limit)
                    .offset(offset)
                    .load::<FileWithMetadataDto>(&mut conn)?;

                let items = entities
                    .into_iter()
                    .map(|dto| FileWithMetadata {
                        category_name: dto.category_name,
                        drive_name: dto.drive_name,
                        path: dto.path,
                        size_bytes: dto.weight,
                    })
                    .collect();

                Ok(PaginatedResult { items, total_count })
            })
            .await?
    }

    async fn count_search_results(&self, selected_drive: &Option<Drive>, query: &str) -> Result<i64, RepositoryError> {
        let pool = self.pool.clone();
        let selected_drive = selected_drive.clone();
        let search_pattern = format!("%{}%", query.replace(" ", "_"));
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;
                let count = count(&selected_drive, &search_pattern, &mut conn)?;
                Ok(count)
            })
            .await?
    }
}

fn count(selected_drive: &Option<Drive>, search_pattern: &String, conn: &mut DieselConnection) -> Result<i64, RepositoryError> {
    let count = if let Some(drive) = selected_drive {
        file_entries::table
            .inner_join(drive_entries::table)
            .filter(drive_entries::name.eq(&drive.name))
            .filter(file_entries::path.like(&search_pattern))
            .count()
            .get_result(conn)?
    } else {
        file_entries::table
            .filter(file_entries::path.like(&search_pattern))
            .count()
            .get_result(conn)?
    };
    Ok(count)
}

#[async_trait::async_trait]
impl FileCommandRepository for SqliteFileRepository {
    async fn remove_duplicates(
        &self,
        category: Category,
        drive: Drive,
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
        let drive_name = drive.name;

        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;
                conn.immediate_transaction::<_, RepositoryError, _>(|conn| {
                    let category_id = Self::save_category(category_name, conn)?;

                    let drive_id = Self::save_drive(drive_name, category_id, conn)?;

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
