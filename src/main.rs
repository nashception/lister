#![windows_subsystem = "windows"]
extern crate libsqlite3_sys;

// ============================================================================
// IMPORTS AND DEPENDENCIES
// ============================================================================

use crate::schema::{drive_entries, file_categories, file_entries, settings};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as DieselError;
use diesel::{Associations, Identifiable, Insertable, Queryable, RunQueryDsl, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use humansize::{format_size, DECIMAL};
use iced::keyboard::key::Named;
use iced::widget::scrollable::RelativeOffset;
use iced::widget::{button, column, row, scrollable, text, text_input, Row, Rule, Space};
use iced::window::{icon, Icon, Settings};
use iced::{keyboard, widget, Alignment, Element, Length, Subscription, Task};
use rfd::AsyncFileDialog;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use tokio::runtime::Runtime;
use walkdir::WalkDir;

mod schema;

// ============================================================================
// CONSTANTS AND GLOBALS
// ============================================================================

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
const ITEMS_PER_PAGE: usize = 100;
const CACHED_SIZE: i64 = 10000;

type DieselPool = Pool<ConnectionManager<SqliteConnection>>;

static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .expect("failed to build Tokio runtime")
});

// ============================================================================
// DOMAIN - CORE BUSINESS LOGIC (HEXAGON CENTER)
// ============================================================================

// Domain Value Objects
#[derive(Clone, Debug, PartialEq)]
pub struct Language {
    code: String,
}

impl Language {
    pub fn new(code: &str) -> Self {
        let normalized_code = match code.to_lowercase().as_str() {
            "en" | "english" => "en",
            "fr" | "french" => "fr",
            _ => "en",
        };
        Self {
            code: normalized_code.to_string(),
        }
    }

    pub fn english() -> Self {
        Self::new("en")
    }

    pub fn french() -> Self {
        Self::new("fr")
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn toggle(&self) -> Self {
        match self.code.as_str() {
            "en" => Self::french(),
            "fr" => Self::english(),
            _ => Self::english(),
        }
    }
}

// Domain Entities
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub size_bytes: i64,
}

#[derive(Clone, Debug)]
pub struct FileWithMetadata {
    pub category_name: String,
    pub drive_name: String,
    pub path: String,
    pub size_bytes: i64,
}

impl FileWithMetadata {
    pub fn parent_directory(&self) -> String {
        Path::new(&self.path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn filename(&self) -> String {
        Path::new(&self.path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct Category {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Drive {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total_count: i64,
}

// Domain Services
pub struct DirectoryScanner {
    pub directory: PathBuf,
}

impl DirectoryScanner {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub async fn scan_directory(&self) -> Vec<FileEntry> {
        let directory = self.directory.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                WalkDir::new(&directory)
                    .sort_by_file_name()
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .map(|e| Self::extract_file_info(&directory, e.path()))
                    .collect()
            })
            .await
            .unwrap()
    }

    fn extract_file_info(base_directory: &PathBuf, file_path: &Path) -> FileEntry {
        FileEntry {
            path: Self::relative_path(base_directory, file_path),
            size_bytes: Self::file_size(file_path),
        }
    }

    fn relative_path(base_directory: &PathBuf, file_path: &Path) -> String {
        file_path
            .strip_prefix(base_directory)
            .expect("File not under chosen directory")
            .to_path_buf()
            .to_string_lossy()
            .into_owned()
    }

    fn file_size(path: &Path) -> i64 {
        fs::metadata(path)
            .expect("Cannot access file metadata")
            .len() as i64
    }
}

// ============================================================================
// PORTS - INTERFACES (HEXAGON BOUNDARIES)
// ============================================================================

// Primary Ports (Driving Side - Used by external actors)

#[async_trait::async_trait]
pub trait FileQueryUseCase: Send + Sync {
    async fn search_files(
        &self,
        query: &str,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult<FileWithMetadata>, DomainError>;

    async fn list_files(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult<FileWithMetadata>, DomainError>;

    async fn count_search_results(&self, query: &str) -> Result<i64, DomainError>;
}

#[async_trait::async_trait]
pub trait FileIndexingUseCase: Send + Sync {
    async fn scan_directory(&self, directory: PathBuf) -> Result<Vec<FileEntry>, DomainError>;
    async fn insert_in_database(
        &self,
        category: String,
        drive: String,
        files: Vec<FileEntry>,
    ) -> Result<usize, DomainError>;
}

#[async_trait::async_trait]
pub trait LanguageManagementUseCase: Send + Sync {
    fn get_current_language(&self) -> Result<Language, DomainError>;
    fn set_language(&self, language: Language) -> Result<(), DomainError>;
    fn load_translations(
        &self,
        language: &Language,
    ) -> Result<HashMap<String, String>, DomainError>;
}

// Secondary Ports (Driven Side - Implemented by external adapters)

#[async_trait::async_trait]
pub trait FileQueryRepository: Send + Sync {
    async fn find_files_paginated(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<PaginatedResult<FileWithMetadata>, RepositoryError>;

    async fn search_files_paginated(
        &self,
        query: &str,
        offset: i64,
        limit: i64,
    ) -> Result<PaginatedResult<FileWithMetadata>, RepositoryError>;

    async fn count_search_results(&self, query: &str) -> Result<i64, RepositoryError>;
}

#[async_trait::async_trait]
pub trait FileCommandRepository: Send + Sync {
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

#[async_trait::async_trait]
pub trait DirectoryPicker: Send + Sync {
    async fn pick_directory(&self) -> Option<PathBuf>;
}

pub trait TranslationLoader: Send + Sync {
    fn load_translations(&self, language: &Language) -> HashMap<String, String>;
}

// ============================================================================
// DOMAIN SERVICES (HEXAGON CORE)
// ============================================================================

pub struct FileQueryService {
    query_repo: Arc<dyn FileQueryRepository>,
}

impl FileQueryService {
    pub fn new(query_repo: Arc<dyn FileQueryRepository>) -> Self {
        Self { query_repo }
    }
}

#[async_trait::async_trait]
impl FileQueryUseCase for FileQueryService {
    async fn search_files(
        &self,
        query: &str,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult<FileWithMetadata>, DomainError> {
        let offset = (page * page_size) as i64;
        let limit = page_size as i64;

        // Optimize small result sets by caching
        let count = self.query_repo.count_search_results(query).await?;
        if count <= CACHED_SIZE {
            self.query_repo
                .search_files_paginated(query, 0, count)
                .await
                .map(|mut result| {
                    let start = offset as usize;
                    let end = (start + page_size).min(result.items.len());
                    result.items = if start < result.items.len() {
                        result.items[start..end].to_vec()
                    } else {
                        Vec::new()
                    };
                    result
                })
                .map_err(DomainError::Repository)
        } else {
            self.query_repo
                .search_files_paginated(query, offset, limit)
                .await
                .map_err(DomainError::Repository)
        }
    }

    async fn list_files(
        &self,
        page: usize,
        page_size: usize,
    ) -> Result<PaginatedResult<FileWithMetadata>, DomainError> {
        let offset = (page * page_size) as i64;
        let limit = page_size as i64;
        self.query_repo
            .find_files_paginated(offset, limit)
            .await
            .map_err(DomainError::Repository)
    }

    async fn count_search_results(&self, query: &str) -> Result<i64, DomainError> {
        self.query_repo
            .count_search_results(query)
            .await
            .map_err(DomainError::Repository)
    }
}

pub struct FileIndexingService {
    command_repo: Arc<dyn FileCommandRepository>,
}

impl FileIndexingService {
    pub fn new(command_repo: Arc<dyn FileCommandRepository>) -> Self {
        Self { command_repo }
    }
}

#[async_trait::async_trait]
impl FileIndexingUseCase for FileIndexingService {
    async fn scan_directory(&self, directory: PathBuf) -> Result<Vec<FileEntry>, DomainError> {
        let scanner = DirectoryScanner::new(directory);
        let files = scanner.scan_directory().await;
        Ok(files)
    }

    async fn insert_in_database(
        &self,
        category: String,
        drive: String,
        files: Vec<FileEntry>,
    ) -> Result<usize, DomainError> {
        let files_count = self
            .command_repo
            .save(Category { name: category }, Drive { name: drive }, files)
            .await?;
        Ok(files_count)
    }
}

pub struct LanguageService {
    language_repo: Arc<dyn LanguageRepository>,
    translation_loader: Arc<dyn TranslationLoader>,
}

impl LanguageService {
    pub fn new(
        language_repo: Arc<dyn LanguageRepository>,
        translation_loader: Arc<dyn TranslationLoader>,
    ) -> Self {
        Self {
            language_repo,
            translation_loader,
        }
    }
}

impl LanguageManagementUseCase for LanguageService {
    fn get_current_language(&self) -> Result<Language, DomainError> {
        self.language_repo
            .get_language()
            .map_err(DomainError::Repository)
    }

    fn set_language(&self, language: Language) -> Result<(), DomainError> {
        self.language_repo
            .set_language(&language)
            .map_err(DomainError::Repository)
    }

    fn load_translations(
        &self,
        language: &Language,
    ) -> Result<HashMap<String, String>, DomainError> {
        Ok(self.translation_loader.load_translations(language))
    }
}

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },
    #[error("Operation failed: {message}")]
    OperationFailed { message: String },
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] DieselError),
    #[error("Connection pool error: {0}")]
    ConnectionPool(#[from] PoolError),
    #[error("Not found")]
    NotFound,
}

// ============================================================================
// SECONDARY ADAPTERS - INFRASTRUCTURE (DRIVEN SIDE)
// ============================================================================

// Database Entities (Infrastructure concern)
#[derive(Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(table_name = file_categories)]
struct FileCategoryEntity {
    id: i32,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(belongs_to(FileCategoryEntity, foreign_key = category_id))]
#[diesel(table_name = drive_entries)]
struct DriveEntryEntity {
    id: i32,
    category_id: i32,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(DriveEntryEntity, foreign_key = drive_id))]
#[diesel(table_name = file_entries)]
struct FileEntryEntity {
    id: i32,
    drive_id: i32,
    path: String,
    weight: i64,
}

#[derive(Queryable)]
struct FileWithMetadataDto {
    category_name: String,
    drive_name: String,
    path: String,
    weight: i64,
}

#[derive(Insertable)]
#[diesel(table_name = file_categories)]
struct NewFileCategoryDto {
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = drive_entries)]
struct NewDriveEntryDto {
    category_id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = file_entries)]
struct NewFileEntryDto {
    drive_id: i32,
    path: String,
    weight: i64,
}

// Database Repository Implementation
pub struct SqliteFileRepository {
    pool: DieselPool,
}

impl SqliteFileRepository {
    pub fn new(database_url: &str) -> Self {
        let pool = Self::create_pool(database_url);
        Self::enable_foreign_keys(&pool);
        Self::run_migrations(&pool);
        Self { pool }
    }

    fn create_pool(database_url: &str) -> DieselPool {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        Pool::builder()
            .build(manager)
            .expect("Failed to create SQLite pool")
    }

    fn enable_foreign_keys(pool: &DieselPool) {
        let conn = &mut pool.get().expect("Failed to get connection");
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(conn)
            .expect("Failed to enable foreign keys");
    }

    fn run_migrations(pool: &DieselPool) {
        let mut conn = pool.get().expect("Failed to get connection");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Migration failed");
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
}

#[async_trait::async_trait]
impl FileQueryRepository for SqliteFileRepository {
    async fn find_files_paginated(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<PaginatedResult<FileWithMetadata>, RepositoryError> {
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
            .await
            .unwrap()
    }

    async fn search_files_paginated(
        &self,
        query: &str,
        offset: i64,
        limit: i64,
    ) -> Result<PaginatedResult<FileWithMetadata>, RepositoryError> {
        let pool = self.pool.clone();
        let search_pattern = format!("%{}%", query.replace(" ", "_"));
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;

                let total_count: i64 = file_entries::table
                    .filter(file_entries::path.like(&search_pattern))
                    .count()
                    .get_result(&mut conn)?;

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
            .await
            .unwrap()
    }

    async fn count_search_results(&self, query: &str) -> Result<i64, RepositoryError> {
        let pool = self.pool.clone();
        let search_pattern = format!("%{}%", query.replace(" ", "_"));
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                let mut conn = pool.get()?;
                let count = file_entries::table
                    .filter(file_entries::path.like(&search_pattern))
                    .count()
                    .get_result(&mut conn)?;
                Ok(count)
            })
            .await
            .unwrap()
    }
}

#[async_trait::async_trait]
impl FileCommandRepository for SqliteFileRepository {
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
            .await
            .unwrap()
    }
}

// Implement language repository interface
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

// Translation Loader Implementation
pub struct JsonTranslationLoader;

impl TranslationLoader for JsonTranslationLoader {
    fn load_translations(&self, language: &Language) -> HashMap<String, String> {
        let data = match language.code() {
            "fr" => include_str!("../translations/fr.json"),
            "en" | _ => include_str!("../translations/en.json"),
        };
        serde_json::from_str(data).unwrap_or_default()
    }
}

// Directory Picker Implementation
pub struct NativeDirectoryPicker;

#[async_trait::async_trait]
impl DirectoryPicker for NativeDirectoryPicker {
    async fn pick_directory(&self) -> Option<PathBuf> {
        AsyncFileDialog::new()
            .set_title("Select Directory to Index")
            .pick_folder()
            .await
            .map(|handle| handle.path().to_path_buf())
    }
}

// ============================================================================
// PRIMARY ADAPTERS - USER INTERFACE (DRIVING SIDE)
// ============================================================================

// Application Messages (UI Events)
#[derive(Clone, Debug)]
enum AppMessage {
    ChangeLanguage(Language),
    LanguageChanged(Language, HashMap<String, String>),
    GoToRead,
    GoToWrite,
    Read(ReadMessage),
    Write(WriteMessage),
    TabPressed { shift: bool },
}

#[derive(Clone, Debug)]
enum ReadMessage {
    FirstPage,
    PrevPage,
    PageInputChanged(String),
    PageInputSubmit,
    NextPage,
    LastPage,
    SearchSubmit,
    ContentChanged(String),
    SearchClear,
    FilesLoaded((u64, PaginatedResult<FileWithMetadata>)),
}

#[derive(Clone, Debug)]
enum WriteMessage {
    CategoryChanged(String),
    DriveChanged(String),
    DirectoryPressed,
    DirectoryChanged(Option<PathBuf>),
    WriteSubmit,
    ScanDirectoryFinished(Vec<FileEntry>),
    InsertInDatabaseFinished(usize),
    ResetForm,
}

// UI Application State
enum Page {
    Read(ReadPage),
    Write(WritePage),
}

// Translation Helper
macro_rules! tr {
    ($translations:expr, $key:expr) => {
        tr_impl($translations, $key, &[])
    };
    ($translations:expr, $key:expr, $( $k:expr => $v:expr ),* ) => {
        tr_impl($translations, $key, &[ $( ($k, $v) ),* ])
    };
}

fn tr_impl(translations: &HashMap<String, String>, key: &str, params: &[(&str, &str)]) -> String {
    let mut text = translations
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string());

    for (k, v) in params {
        text = text.replace(&format!("{{{}}}", k), v);
    }

    text
}

// Read Page (File Listing and Search)
struct ReadPage {
    query_use_case: Arc<dyn FileQueryUseCase>,
    search_query: String,
    current_files: Vec<FileWithMetadata>,
    cached_query: Option<String>,
    cached_results: Option<Vec<FileWithMetadata>>,
    page_input_value: String,
    total_count: i64,
    current_page_index: usize,
    active_task_id: u64,
    search_input_id: text_input::Id,
    scroll_bar_id: scrollable::Id,
}

impl ReadPage {
    fn new(query_use_case: Arc<dyn FileQueryUseCase>) -> (Self, Task<ReadMessage>) {
        let mut page = Self {
            query_use_case,
            search_query: String::new(),
            current_files: Vec::new(),
            cached_query: None,
            cached_results: None,
            page_input_value: String::new(),
            total_count: 0,
            current_page_index: 0,
            active_task_id: 0,
            search_input_id: text_input::Id::unique(),
            scroll_bar_id: scrollable::Id::unique(),
        };
        let task = page.load_current_page();
        (page, task)
    }

    fn title(&self, translations: &HashMap<String, String>) -> String {
        tr!(translations, "read_page_title")
    }

    fn load_current_page(&mut self) -> Task<ReadMessage> {
        // Use cached results if available
        if let (Some(cached), Some(query)) = (&self.cached_results, &self.cached_query) {
            if *query == self.search_query {
                let start = self.current_page_index * ITEMS_PER_PAGE;
                let end = (start + ITEMS_PER_PAGE).min(cached.len());
                self.current_files = cached[start..end].to_vec();
                self.total_count = cached.len() as i64;
                return Task::none();
            }
        }

        self.active_task_id += 1;
        let task_id = self.active_task_id;
        let search_query = self.search_query.clone();
        let query_use_case = self.query_use_case.clone();
        let page = self.current_page_index;

        Task::perform(
            async move {
                let result = if search_query.is_empty() {
                    query_use_case.list_files(page, ITEMS_PER_PAGE).await
                } else {
                    query_use_case
                        .search_files(&search_query, page, ITEMS_PER_PAGE)
                        .await
                };
                (task_id, result)
            },
            |(finished_task_id, result)| {
                ReadMessage::FilesLoaded((finished_task_id, result.unwrap()))
            },
        )
    }

    fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, ReadMessage> {
        let search_section = self.search_section(translations);
        let files = self.files_section();
        let pagination_section = self.pagination_section(translations);

        column![search_section, files, pagination_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn search_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, ReadMessage> {
        let search_input = text_input(&tr!(translations, "search_placeholder"), &self.search_query)
            .id(self.search_input_id.clone())
            .on_input(ReadMessage::ContentChanged)
            .on_submit(ReadMessage::SearchSubmit)
            .padding(10)
            .width(Length::Fill);

        let search_button = button(text(tr!(translations, "search_button")))
            .on_press(ReadMessage::SearchSubmit)
            .padding(10);

        let clear_button = button(text(tr!(translations, "clear_button")))
            .on_press(ReadMessage::SearchClear)
            .padding(10)
            .style(button::secondary);

        column![row![search_input, search_button, clear_button].spacing(10)].into()
    }

    fn files_section(&'_ self) -> Element<'_, ReadMessage> {
        let file_rows: Vec<Element<'_, ReadMessage>> = self
            .current_files
            .iter()
            .map(|file| {
                row![
                    text(&file.category_name).width(Length::FillPortion(1)),
                    text(&file.drive_name).width(Length::FillPortion(2)),
                    text(file.parent_directory()).width(Length::FillPortion(3)),
                    text(file.filename()).width(Length::FillPortion(4)),
                    text(format_size(file.size_bytes as u64, DECIMAL))
                        .width(Length::FillPortion(1))
                ]
                .padding(3)
                .into()
            })
            .collect();

        column![
            Rule::horizontal(1),
            scrollable(column(file_rows))
                .id(self.scroll_bar_id.clone())
                .height(Length::Fill),
            Rule::horizontal(1),
        ]
        .into()
    }

    fn pagination_section(
        &self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, ReadMessage> {
        let total_pages = self.total_pages();

        let first_button = button(text(tr!(translations, "first_button")))
            .on_press_maybe(if self.current_page_index > 0 {
                Some(ReadMessage::FirstPage)
            } else {
                None
            })
            .padding(8)
            .style(if self.current_page_index > 0 {
                button::secondary
            } else {
                button::text
            });

        let prev_button = button(text(tr!(translations, "prev_button")))
            .on_press_maybe(if self.current_page_index > 0 {
                Some(ReadMessage::PrevPage)
            } else {
                None
            })
            .padding(8)
            .style(if self.current_page_index > 0 {
                button::secondary
            } else {
                button::text
            });

        let page_info = text(format!(
            "{:^5} / {:^5} - {:^7}",
            self.current_page_index + 1,
            total_pages,
            self.total_count
        ))
        .size(14);

        let next_button = button(text(tr!(translations, "next_button")))
            .on_press_maybe(if self.current_page_index < total_pages.saturating_sub(1) {
                Some(ReadMessage::NextPage)
            } else {
                None
            })
            .padding(8)
            .style(if self.current_page_index < total_pages.saturating_sub(1) {
                button::secondary
            } else {
                button::text
            });

        let last_button = button(text(tr!(translations, "last_button")))
            .on_press_maybe(if self.current_page_index < total_pages.saturating_sub(1) {
                Some(ReadMessage::LastPage)
            } else {
                None
            })
            .padding(8)
            .style(if self.current_page_index < total_pages.saturating_sub(1) {
                button::secondary
            } else {
                button::text
            });

        let page_input = text_input(
            &tr!(translations, "page_placeholder"),
            &self.page_input_value,
        )
        .on_input(ReadMessage::PageInputChanged)
        .on_submit(ReadMessage::PageInputSubmit)
        .padding(8)
        .width(Length::Fixed(100f32));

        row![
            first_button,
            prev_button,
            page_info,
            next_button,
            last_button,
            page_input,
        ]
        .spacing(20)
        .align_y(Alignment::Center)
        .into()
    }

    fn total_pages(&self) -> usize {
        if self.total_count == 0 {
            1
        } else {
            ((self.total_count as usize) + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE
        }
    }

    fn update(&mut self, message: ReadMessage) -> Task<ReadMessage> {
        match message {
            ReadMessage::PrevPage => {
                self.navigate_to_page(self.current_page_index.saturating_sub(1))
            }
            ReadMessage::NextPage => {
                let total_pages = self.total_pages();
                if self.current_page_index + 1 < total_pages {
                    self.navigate_to_page(self.current_page_index + 1)
                } else {
                    Task::none()
                }
            }
            ReadMessage::FirstPage => self.navigate_to_page(0),
            ReadMessage::LastPage => self.navigate_to_page(self.total_pages().saturating_sub(1)),
            ReadMessage::SearchSubmit => {
                self.current_page_index = 0;
                self.load_current_page()
            }
            ReadMessage::SearchClear => {
                self.search_query.clear();
                self.current_page_index = 0;
                self.load_current_page()
            }
            ReadMessage::ContentChanged(content) => {
                self.search_query = content;
                Task::none()
            }
            ReadMessage::PageInputChanged(page_number) => {
                self.page_input_value = page_number;
                Task::none()
            }
            ReadMessage::PageInputSubmit => {
                if let Ok(page) = self.page_input_value.parse::<usize>() {
                    if page > 0 && page <= self.total_pages() {
                        self.navigate_to_page(page - 1)
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            ReadMessage::FilesLoaded((task_id, result)) => {
                self.handle_files_loaded(task_id, result)
            }
        }
    }

    fn navigate_to_page(&mut self, page_index: usize) -> Task<ReadMessage> {
        self.current_page_index = page_index;
        self.load_current_page()
    }

    fn handle_files_loaded(
        &mut self,
        task_id: u64,
        result: PaginatedResult<FileWithMetadata>,
    ) -> Task<ReadMessage> {
        if task_id != self.active_task_id {
            return Task::none();
        }

        self.process_loaded_files(result)
            .chain(text_input::focus(self.search_input_id.clone()))
    }

    fn process_loaded_files(
        &mut self,
        result: PaginatedResult<FileWithMetadata>,
    ) -> Task<ReadMessage> {
        if result.total_count <= CACHED_SIZE && !self.search_query.is_empty() {
            self.cached_results = Some(result.items.clone());
            self.cached_query = Some(self.search_query.clone());
            let start = self.current_page_index * ITEMS_PER_PAGE;
            let end = (start + ITEMS_PER_PAGE).min(result.items.len());
            self.current_files = result.items[start..end].to_vec();
            self.total_count = result.items.len() as i64;
        } else {
            self.current_files = result.items;
            self.total_count = result.total_count;
            self.cached_results = None;
            self.cached_query = None;
        }

        scrollable::snap_to(self.scroll_bar_id.clone(), RelativeOffset::START)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum IndexingState {
    Ready,
    Scanning,
    Saving,
    Completed { files_indexed: usize },
}

// Write Page (Directory Indexing)
struct WritePage {
    indexing_use_case: Arc<dyn FileIndexingUseCase>,
    directory_picker: Arc<dyn DirectoryPicker>,
    category: String,
    drive: String,
    directory: Option<PathBuf>,
    state: IndexingState,
    category_input_id: text_input::Id,
}

impl WritePage {
    fn new(
        indexing_use_case: Arc<dyn FileIndexingUseCase>,
        directory_picker: Arc<dyn DirectoryPicker>,
    ) -> (Self, Task<WriteMessage>) {
        let category_input_id = text_input::Id::unique();
        let page = Self {
            indexing_use_case,
            directory_picker,
            category: String::new(),
            drive: String::new(),
            directory: None,
            state: IndexingState::Ready,
            category_input_id: category_input_id.clone(),
        };
        (page, text_input::focus(category_input_id))
    }

    fn title(&self, translations: &HashMap<String, String>) -> String {
        tr!(translations, "write_page_title")
    }

    fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, WriteMessage> {
        let form_section = self.form_section(translations);
        let action_section = self.action_section(translations);
        let status_section = self.status_section(translations);

        column![form_section, action_section, status_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn form_section(&'_ self, translations: &HashMap<String, String>) -> Element<'_, WriteMessage> {
        let category_input = text_input(&tr!(translations, "category_placeholder"), &self.category)
            .on_input(WriteMessage::CategoryChanged)
            .id(self.category_input_id.clone())
            .padding(10)
            .width(Length::Fill);

        let drive_input = text_input(&tr!(translations, "drive_placeholder"), &self.drive)
            .on_input(WriteMessage::DriveChanged)
            .padding(10)
            .width(Length::Fill);

        let directory_section = self.directory_section(translations);

        column![
            text(tr!(translations, "file_indexing_setup"))
                .size(24)
                .style(text::primary),
            Rule::horizontal(1),
            column![
                text(tr!(translations, "category_label")).size(16),
                category_input,
            ]
            .spacing(5),
            column![text(tr!(translations, "drive_label")).size(16), drive_input,].spacing(5),
            directory_section,
        ]
        .spacing(15)
        .into()
    }

    fn directory_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, WriteMessage> {
        let directory_label = text(tr!(translations, "directory_label")).size(16);

        let directory_display = if let Some(dir) = &self.directory {
            text(tr!(translations, "selected_directory", "dir" => &dir.display().to_string()))
                .style(text::success)
        } else {
            text(tr!(translations, "no_directory_selected")).style(text::secondary)
        };

        let browse_button = button(text(tr!(translations, "browse_directory")))
            .on_press(WriteMessage::DirectoryPressed)
            .padding(10)
            .style(button::secondary);

        column![
            directory_label,
            row![directory_display, browse_button]
                .spacing(10)
                .align_y(Alignment::Center),
        ]
        .spacing(5)
        .into()
    }

    fn action_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, WriteMessage> {
        let can_submit = self.form_is_complete() && self.state == IndexingState::Ready;

        let submit_button = button(text(tr!(translations, "start_indexing")))
            .on_press_maybe(if can_submit {
                Some(WriteMessage::WriteSubmit)
            } else {
                None
            })
            .padding(15)
            .width(Length::Fill)
            .style(if can_submit {
                button::primary
            } else {
                button::text
            });

        let requirements_text = if !self.form_is_complete() {
            text(tr!(translations, "fill_all_fields"))
                .style(text::secondary)
                .size(12)
        } else {
            text("")
        };

        column![Rule::horizontal(1), submit_button, requirements_text,]
            .spacing(10)
            .into()
    }

    fn status_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, WriteMessage> {
        match &self.state {
            IndexingState::Ready => column![].into(),
            IndexingState::Scanning => column![
                text(tr!(translations, "scan_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "scan_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10)
            .into(),
            IndexingState::Saving => column![
                text(tr!(translations, "save_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "save_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10)
            .into(),
            IndexingState::Completed { files_indexed } => column![
                Rule::horizontal(1),
                column![
                    text(tr!(translations, "done_status"))
                        .size(18)
                        .style(text::success),
                    text(
                        tr!(translations, "done_details", "nb_files" => &files_indexed.to_string())
                    )
                    .style(text::success)
                    .size(14),
                    button(text(tr!(translations, "start_new_indexing")))
                        .on_press(WriteMessage::ResetForm)
                        .padding(10)
                        .style(button::secondary),
                ]
                .spacing(10),
            ]
            .spacing(15)
            .into(),
        }
    }

    fn form_is_complete(&self) -> bool {
        !self.category.is_empty() && !self.drive.is_empty() && self.directory.is_some()
    }

    fn update(&mut self, message: WriteMessage) -> Task<WriteMessage> {
        match message {
            WriteMessage::CategoryChanged(value) => {
                self.category = value;
                Task::none()
            }
            WriteMessage::DriveChanged(value) => {
                self.drive = value;
                Task::none()
            }
            WriteMessage::DirectoryPressed => {
                let picker = self.directory_picker.clone();
                Task::perform(
                    async move { picker.pick_directory().await },
                    WriteMessage::DirectoryChanged,
                )
            }
            WriteMessage::DirectoryChanged(directory) => {
                self.directory = directory;
                Task::none()
            }
            WriteMessage::WriteSubmit => self.start_indexing(),
            WriteMessage::ScanDirectoryFinished(scanned_files) => {
                self.insert_in_database(scanned_files)
            }
            WriteMessage::InsertInDatabaseFinished(count) => {
                self.state = IndexingState::Completed {
                    files_indexed: count,
                };
                Task::none()
            }
            WriteMessage::ResetForm => {
                self.category.clear();
                self.drive.clear();
                self.directory = None;
                self.state = IndexingState::Ready;
                text_input::focus(self.category_input_id.clone())
            }
        }
    }

    fn start_indexing(&mut self) -> Task<WriteMessage> {
        if self.state != IndexingState::Ready {
            return Task::none();
        }
        self.state = IndexingState::Scanning;

        let indexing_use_case = self.indexing_use_case.clone();
        let directory = self.directory.clone().unwrap();

        Task::perform(
            async move {
                indexing_use_case
                    .scan_directory(directory)
                    .await
                    .unwrap_or(Vec::new())
            },
            WriteMessage::ScanDirectoryFinished,
        )
    }

    fn insert_in_database(&mut self, files: Vec<FileEntry>) -> Task<WriteMessage> {
        if self.state != IndexingState::Scanning {
            return Task::none();
        }
        self.state = IndexingState::Saving;

        let indexing_use_case = self.indexing_use_case.clone();
        let category = self.category.clone();
        let drive = self.drive.clone();

        Task::perform(
            async move {
                indexing_use_case
                    .insert_in_database(category, drive, files)
                    .await
                    .unwrap_or(0)
            },
            WriteMessage::InsertInDatabaseFinished,
        )
    }
}

// Main Application
struct ListerApp {
    query_use_case: Arc<dyn FileQueryUseCase>,
    indexing_use_case: Arc<dyn FileIndexingUseCase>,
    language_use_case: Arc<dyn LanguageManagementUseCase>,
    directory_picker: Arc<dyn DirectoryPicker>,
    current_language: Language,
    translations: HashMap<String, String>,
    current_page: Page,
}

impl ListerApp {
    fn new() -> (Self, Task<AppMessage>) {
        // Create the single repository instance
        let repository = Arc::new(SqliteFileRepository::new("app.db"));
        let translation_loader = Arc::new(JsonTranslationLoader);
        let directory_picker = Arc::new(NativeDirectoryPicker);

        let query_service = Arc::new(FileQueryService::new(repository.clone()));
        let indexing_service = Arc::new(FileIndexingService::new(repository.clone()));
        let language_service =
            Arc::new(LanguageService::new(repository.clone(), translation_loader));

        let current_language = language_service
            .get_current_language()
            .unwrap_or_else(|_| Language::english());
        let translations = language_service
            .load_translations(&current_language)
            .unwrap_or_default();

        let (read_page, task) = ReadPage::new(query_service.clone());

        (
            Self {
                query_use_case: query_service,
                indexing_use_case: indexing_service,
                language_use_case: language_service,
                directory_picker,
                current_language,
                translations,
                current_page: Page::Read(read_page),
            },
            task.map(AppMessage::Read),
        )
    }

    fn title(&self) -> String {
        match &self.current_page {
            Page::Read(page) => page.title(&self.translations),
            Page::Write(page) => page.title(&self.translations),
        }
    }

    fn view(&'_ self) -> Element<'_, AppMessage> {
        let language_toggle = self.language_toggle();
        let nav_bar = self.nav_bar();

        let content = match &self.current_page {
            Page::Read(page) => page.view(&self.translations).map(AppMessage::Read),
            Page::Write(page) => page.view(&self.translations).map(AppMessage::Write),
        };

        column![language_toggle, Space::with_height(10), nav_bar, content]
            .padding(20)
            .into()
    }

    fn nav_bar(&'_ self) -> Row<'_, AppMessage> {
        row![
            button(text(tr!(&self.translations, "read_page")).align_x(Alignment::Center))
                .on_press(AppMessage::GoToRead)
                .style(match &self.current_page {
                    Page::Read(_) => button::primary,
                    Page::Write(_) => button::secondary,
                })
                .width(Length::Fill),
            button(text(tr!(&self.translations, "write_page")).align_x(Alignment::Center))
                .on_press(AppMessage::GoToWrite)
                .style(match &self.current_page {
                    Page::Read(_) => button::secondary,
                    Page::Write(_) => button::primary,
                })
                .width(Length::Fill)
        ]
        .spacing(10)
    }

    fn language_toggle(&'_ self) -> Row<'_, AppMessage> {
        let label = match self.current_language.code() {
            "fr" => "FR",
            _ => "EN",
        };

        let toggle_button = button(text(label))
            .on_press(AppMessage::ChangeLanguage(self.current_language.toggle()));

        row![Space::with_width(Length::Fill), toggle_button].width(Length::Fill)
    }

    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::ChangeLanguage(language) => self.change_language(language),
            AppMessage::LanguageChanged(language, translations) => {
                self.current_language = language;
                self.translations = translations;
                Task::none()
            }
            AppMessage::GoToRead => {
                if matches!(self.current_page, Page::Write(_)) {
                    let (read_page, task) = ReadPage::new(self.query_use_case.clone());
                    self.current_page = Page::Read(read_page);
                    task.map(AppMessage::Read)
                } else {
                    Task::none()
                }
            }
            AppMessage::GoToWrite => {
                if matches!(self.current_page, Page::Read(_)) {
                    let (write_page, task) = WritePage::new(
                        self.indexing_use_case.clone(),
                        self.directory_picker.clone(),
                    );
                    self.current_page = Page::Write(write_page);
                    task.map(AppMessage::Write)
                } else {
                    Task::none()
                }
            }
            AppMessage::Read(msg) => {
                if let Page::Read(page) = &mut self.current_page {
                    page.update(msg).map(AppMessage::Read)
                } else {
                    Task::none()
                }
            }
            AppMessage::Write(msg) => {
                if let Page::Write(page) = &mut self.current_page {
                    page.update(msg).map(AppMessage::Write)
                } else {
                    Task::none()
                }
            }
            AppMessage::TabPressed { shift } => {
                if shift {
                    widget::focus_previous()
                } else {
                    widget::focus_next()
                }
            }
        }
    }

    fn change_language(&mut self, language: Language) -> Task<AppMessage> {
        let language_use_case = self.language_use_case.clone();
        Task::perform(
            async move {
                language_use_case.set_language(language.clone()).ok();
                let translations = language_use_case
                    .load_translations(&language)
                    .unwrap_or_default();
                (language, translations)
            },
            |(language, translations)| AppMessage::LanguageChanged(language, translations),
        )
    }

    fn subscription(&self) -> Subscription<AppMessage> {
        keyboard::on_key_press(|key, modifiers| {
            let keyboard::Key::Named(key) = key else {
                return None;
            };
            match (key, modifiers) {
                (Named::Tab, _) => Some(AppMessage::TabPressed {
                    shift: modifiers.shift(),
                }),
                _ => None,
            }
        })
    }
}

// ============================================================================
// APPLICATION ENTRY POINT
// ============================================================================

fn window() -> Settings {
    Settings {
        icon: Some(lister_icon()),
        ..Default::default()
    }
}

fn lister_icon() -> Icon {
    icon::from_file_data(include_bytes!("../assets/icon.png"), None)
        .expect("Icon file should exist and be ICO format")
}

fn main() -> iced::Result {
    iced::application(ListerApp::title, ListerApp::update, ListerApp::view)
        .subscription(ListerApp::subscription)
        .window(window())
        .run_with(ListerApp::new)
}
