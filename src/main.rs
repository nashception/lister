extern crate libsqlite3_sys;

use crate::schema::{drive_entries, file_categories, file_entries};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as DieselError;
use diesel::ExpressionMethods;
use diesel::{Associations, Identifiable, Insertable, Queryable, RunQueryDsl, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use humansize::{format_size, DECIMAL};
use iced::widget::{button, column, row, scrollable, text, Rule};
use iced::{Alignment, Element, Length};
use std::path::Path;
use std::sync::Arc;

mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

const ITEMS_PER_PAGE: usize = 500;

type DieselPool = Pool<ConnectionManager<SqliteConnection>>;

fn main() -> iced::Result {
    println!("Hello, world!");
    iced::run("Lister", ListerApp::update, ListerApp::view)
}

#[derive(Clone, Debug)]
enum AppMessage {
    GoToRead,
    GoToWrite,
    Read(ReadMessage),
    Write(WriteMessage),
}

enum Page {
    Read(ReadPage),
    Write(WritePage),
}

#[derive(Clone, Debug)]
enum ReadMessage {
    SearchPrevPage,
    SearchNextPage,
}

#[derive(Clone, Debug)]
enum WriteMessage {
    Ok(usize),
}

struct ReadPage {
    service: Arc<ListerService>,

    all_files: Vec<FileEntryModel>,
    filtered_indices: Vec<usize>,
    current_page_index: usize,
}

struct WritePage {
    service: Arc<ListerService>,
}

impl ReadPage {
    fn new(service: Arc<ListerService>) -> Self {
        let all_files = service.find_all_files().expect("Error finding all files");
        let filtered_indices = (0..all_files.len()).collect();
        Self {
            service,
            all_files,
            filtered_indices,
            current_page_index: 0,
        }
    }

    fn view(&'_ self) -> Element<'_, ReadMessage> {
        let files = self.files();
        let pagination_section = self.create_pagination_section();


        column![files, pagination_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn files(&'_ self) -> Element<'_, ReadMessage> {
        let start = self.current_page_index * ITEMS_PER_PAGE;
        let end = (start + ITEMS_PER_PAGE).min(self.filtered_indices.len());
        let files_to_show = &self.filtered_indices[start..end];
        let file_rows: Vec<Element<'_, ReadMessage>> = files_to_show
            .iter()
            .map(|&i| {
                let file = &self.all_files[i];
                row![
                    text(&file.category_name).width(Length::FillPortion(1)),
                    text(&file.drive_name).width(Length::FillPortion(2)),
                    text(file.parent_dir()).width(Length::FillPortion(3)),
                    text(file.file_name()).width(Length::FillPortion(4)),
                    text(format_size(file.weight as u64, DECIMAL)).width(Length::FillPortion(1))
                ]
                .padding(3)
                .into()
            })
            .collect();
        column![
            Rule::horizontal(1),
            scrollable(column(file_rows)).height(Length::Fill),
            Rule::horizontal(1),
        ]
        .into()
    }

    fn create_pagination_section(&self) -> Element<'_, ReadMessage> {
        let total_pages = self.total_pages();

        let prev_button = button("Prev")
            .on_press_maybe(if self.current_page_index > 0 {
                Some(ReadMessage::SearchPrevPage)
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
            self.filtered_indices.len()
        ))
        .size(14);

        let next_button = button("Next")
            .on_press_maybe(if self.current_page_index < total_pages - 1 {
                Some(ReadMessage::SearchNextPage)
            } else {
                None
            })
            .padding(8)
            .style(if self.current_page_index < total_pages - 1 {
                button::secondary
            } else {
                button::text
            });

        row![prev_button, page_info, next_button]
            .spacing(20)
            .align_y(Alignment::Center)
            .into()
    }

    fn total_pages(&self) -> usize {
        let total_items = if self.filtered_indices.is_empty() {
            1
        } else if self.filtered_indices.is_empty() {
            self.all_files.len()
        } else {
            self.filtered_indices.len()
        };

        (total_items + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE.max(1)
    }

    pub fn next_page(&mut self) {
        let total_pages = self.total_pages();
        if self.current_page_index + 1 < total_pages {
            self.current_page_index += 1;
        }
    }

    pub fn previous_page(&mut self) {
        if self.current_page_index > 0 {
            self.current_page_index -= 1;
        }
    }

    fn update(&mut self, message: ReadMessage) {
        match message {
            ReadMessage::SearchPrevPage => self.previous_page(),
            ReadMessage::SearchNextPage => self.next_page(),
        }
    }
}

impl WritePage {
    fn new(service: Arc<ListerService>) -> Self {
        Self { service }
    }

    fn view(&'_ self) -> Element<'_, WriteMessage> {
        text("Error").into()
    }

    fn update(&mut self, message: WriteMessage) {
        println!("{:?}", message)
    }
}

struct ListerApp {
    service: Arc<ListerService>,

    current_page: Page,
}

impl Default for ListerApp {
    fn default() -> Self {
        let service = init_back_end();
        Self {
            service: service.clone(),
            current_page: Page::Read(ReadPage::new(service)),
        }
    }
}

fn init_back_end() -> Arc<ListerService> {
    let pool = get_connection_pool("app.db");
    Arc::new(ListerService::new(ListerRepository::new(pool)))
}

impl ListerApp {
    fn view(&'_ self) -> Element<'_, AppMessage> {
        let nav_bar = row![
            button("Read").on_press(AppMessage::GoToRead),
            button("Write").on_press(AppMessage::GoToWrite)
        ];

        let content = match &self.current_page {
            Page::Read(page) => page.view().map(AppMessage::Read),
            Page::Write(page) => page.view().map(AppMessage::Write),
        };

        column![nav_bar, content].padding(20).into()
    }

    fn update(&mut self, message: AppMessage) {
        match &mut self.current_page {
            Page::Read(page) => match message {
                AppMessage::GoToWrite => {
                    let write_page = WritePage::new(self.service.clone());
                    self.current_page = Page::Write(write_page)
                }
                AppMessage::Read(msg) => page.update(msg),
                _ => {}
            },
            Page::Write(page) => match message {
                AppMessage::GoToRead => {
                    let read_page = ReadPage::new(self.service.clone());
                    self.current_page = Page::Read(read_page);
                }
                AppMessage::Write(msg) => page.update(msg),
                _ => {}
            },
        }
    }
}

fn get_connection_pool(database_url: &str) -> DieselPool {
    let pool = create_pool(database_url);
    enable_foreign_keys_constraints(&pool);
    run_migrations(&pool);
    pool
}

fn create_pool(database_url: &str) -> Pool<ConnectionManager<SqliteConnection>> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create SQLite pool");
    pool
}

fn enable_foreign_keys_constraints(pool: &Pool<ConnectionManager<SqliteConnection>>) {
    let conn = &mut pool.get().expect("Failed to get connection from pool");
    diesel::sql_query("PRAGMA foreign_keys = ON")
        .execute(conn)
        .expect("Failed to enable foreign keys");
}

fn run_migrations(pool: &DieselPool) {
    let mut conn = pool.get().expect("Failed to get connection from pool");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Migration failed");
}

#[test]
fn lister_repository_with_database() {
    let pool = get_connection_pool("file:memdb1?mode=memory&cache=shared");

    let new_cat = NewFileCategory { name: "Cat Videos" };
    let another_new_cat = NewFileCategory { name: "Games" };
    let new_drive = NewDriveEntry {
        category_id: 2,
        name: "Windows Drive",
    };
    let another_new_drive = NewDriveEntry {
        category_id: 2,
        name: "Linux Drive",
    };
    let lister_repository = ListerRepository::new(pool);

    let new_category_id = lister_repository.add_category(new_cat).unwrap();
    let another_new_category_id = lister_repository.add_category(another_new_cat).unwrap();

    let rows = lister_repository.find_all_categories().unwrap();

    let expected = vec![
        FileCategoryEntity {
            id: new_category_id,
            name: "Cat Videos".into(),
        },
        FileCategoryEntity {
            id: another_new_category_id,
            name: "Games".into(),
        },
    ];

    assert_eq!(rows, expected);

    let new_drive_id = lister_repository.add_drive(new_drive).unwrap();
    let another_new_drive_id = lister_repository.add_drive(another_new_drive).unwrap();

    let rows = lister_repository.find_all_drives_by_category_id(2).unwrap();

    let expected = vec![
        DriveEntryEntity {
            id: new_drive_id,
            category_id: 2,
            name: "Windows Drive".into(),
        },
        DriveEntryEntity {
            id: another_new_drive_id,
            category_id: 2,
            name: "Linux Drive".into(),
        },
    ];

    assert_eq!(rows, expected);
}

#[derive(Clone)]
struct ListerRepository {
    pool: DieselPool,
}

impl ListerRepository {
    fn new(pool: DieselPool) -> Self {
        ListerRepository { pool }
    }
}

#[derive(Debug, thiserror::Error)]
enum RepositoryError {
    #[error("DB error: {0}")]
    Diesel(#[from] DieselError),

    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
}

type RepositoryResult<T> = Result<T, RepositoryError>;

impl ListerRepository {
    fn add_category(&self, category: NewFileCategory<'_>) -> RepositoryResult<i32> {
        let mut conn = self.pool.get()?;
        let id = diesel::insert_into(file_categories::table)
            .values(category)
            .returning(file_categories::id)
            .get_result(&mut conn)?;
        Ok(id)
    }

    fn add_drive(&self, drive: NewDriveEntry<'_>) -> RepositoryResult<i32> {
        let mut conn = self.pool.get()?;
        let id = diesel::insert_into(drive_entries::table)
            .values(drive)
            .returning(drive_entries::id)
            .get_result(&mut conn)?;
        Ok(id)
    }

    fn add_files(&self, files: Vec<NewFileEntry<'_>>) -> RepositoryResult<()> {
        let mut conn = self.pool.get()?;
        conn.immediate_transaction::<_, RepositoryError, _>(|conn| {
            diesel::insert_into(file_entries::table)
                .values(&files)
                .execute(conn)?;
            Ok(())
        })?;
        Ok(())
    }

    fn find_all_files(&self) -> RepositoryResult<Vec<FileWithInfo>> {
        let mut conn = self.pool.get()?;
        let files = file_entries::table
            .inner_join(drive_entries::table.inner_join(file_categories::table))
            .select((
                file_categories::name,
                drive_entries::name,
                file_entries::path,
                file_entries::weight,
                ))
            .load(&mut conn)?;
        Ok(files)
    }

    fn find_all_categories(&self) -> RepositoryResult<Vec<FileCategoryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = file_categories::table.load(&mut conn)?;
        Ok(rows)
    }

    fn find_all_drives_by_category_id(
        &self,
        category_id: i32,
    ) -> RepositoryResult<Vec<DriveEntryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = drive_entries::table
            .filter(drive_entries::category_id.eq(category_id))
            .load(&mut conn)?;
        Ok(rows)
    }

    fn find_all_files_by_drive_id(&self, drive_id: i32) -> RepositoryResult<Vec<FileEntryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = file_entries::table
            .filter(file_entries::drive_id.eq(drive_id))
            .load::<FileEntryEntity>(&mut conn)?;
        Ok(rows)
    }
}

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
struct FileWithInfo {
    category_name: String,
    drive_name: String,
    path: String,
    weight: i64,
}

#[derive(Insertable)]
#[diesel(table_name = file_categories)]
struct NewFileCategory<'a> {
    name: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = drive_entries)]
struct NewDriveEntry<'a> {
    category_id: i32,
    name: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = file_entries)]
struct NewFileEntry<'a> {
    drive_id: i32,
    path: &'a str,
    weight: i64,
}

#[derive(Debug, Clone, PartialEq)]
struct FileCategoryModel {
    id: i32,
    name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct DriveEntryModel {
    id: i32,
    name: String,
}

#[derive(Debug, Clone, PartialEq)]
struct FileEntryModel {
    category_name: String,
    drive_name: String,
    path: String,
    weight: i64,
}

impl FileEntryModel {
    pub fn parent_dir(&self) -> String {
        Path::new(&self.path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "".to_string())
    }

    pub fn file_name(&self) -> String {
        Path::new(&self.path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| "".to_string())
    }
}

#[derive(Debug, thiserror::Error)]
enum ServiceError {
    #[error("Repository error: {0}")]
    Repo(#[from] RepositoryError),
}

type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Clone)]
struct ListerService {
    repo: ListerRepository,
}

impl ListerService {
    fn new(repo: ListerRepository) -> Self {
        ListerService { repo }
    }

    fn find_all_files(&self) -> ServiceResult<Vec<FileEntryModel>> {
        let entities = self.repo.find_all_files()?;
        Ok(entities.into_iter().map(|e| e.into()).collect())
    }
}

impl From<FileCategoryEntity> for FileCategoryModel {
    fn from(value: FileCategoryEntity) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<DriveEntryEntity> for DriveEntryModel {
    fn from(value: DriveEntryEntity) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<FileWithInfo> for FileEntryModel {
    fn from(value: FileWithInfo) -> Self {
        Self {
            category_name: value.category_name,
            drive_name: value.drive_name,
            path: value.path,
            weight: value.weight,
        }
    }
}
