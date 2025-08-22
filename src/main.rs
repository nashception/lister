extern crate libsqlite3_sys;

use crate::schema::{drive_entries, file_categories, file_entries};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as DieselError;
use diesel::ExpressionMethods;
use diesel::{Associations, Identifiable, Insertable, Queryable, RunQueryDsl, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use humansize::{format_size, DECIMAL};
use iced::widget::{button, column, row, scrollable, text, text_input, Rule};
use iced::{Alignment, Element, Length};
use std::path::Path;
use std::sync::Arc;

mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

const ITEMS_PER_PAGE: usize = 100;
const CACHED_SIZE: i64 = 10000;

type DieselPool = Pool<ConnectionManager<SqliteConnection>>;

#[test]
fn insert_rows() {
    let pool = get_connection_pool("app.db");
    let backing: Vec<String> = (0..14000000)
        .map(|i| format!("Dummy series/Episode {}", i))
        .collect();

    let repository = ListerRepository::new(pool);

    repository.add_category(NewFileCategory { name: "Series" });
    repository.add_drive(NewDriveEntry {
        category_id: 1,
        name: "Windows Drive",
    });

    let files: Vec<NewFileEntry> = backing
        .iter()
        .enumerate()
        .map(|(i, s)| NewFileEntry {
            drive_id: 1,
            path: s.as_str(),
            weight: 200_000 + (i as i64) * 42,
        })
        .collect();
    repository.add_files(files);
}

fn main() -> iced::Result {
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
    FirstPage,
    PrevPage,
    PageInputChanged(String),
    PageInputSubmit,
    NextPage,
    LastPage,
    SearchSubmit,
    ContentChanged(String),
    SearchClear,
}

#[derive(Clone, Debug)]
enum WriteMessage {
    Ok(usize),
}

struct PaginatedFiles {
    files: Vec<FileWithInfoModel>,
    total_count: i64,
}

struct ReadPage {
    service: Arc<ListerService>,
    search_query: String,
    current_files: Vec<FileWithInfoModel>,
    cached_query: Option<String>,
    cached_results: Option<Vec<FileWithInfoModel>>,
    page_input_value: String,
    total_count: i64,
    current_page_index: usize,
}

struct WritePage {
    service: Arc<ListerService>,
}

impl ReadPage {
    fn new(service: Arc<ListerService>) -> Self {
        let mut page = Self {
            service,
            search_query: String::new(),
            current_files: Vec::new(),
            cached_query: None,
            cached_results: None,
            page_input_value: String::new(),
            total_count: 0,
            current_page_index: 0,
        };
        page.load_current_page();
        page
    }

    fn load_current_page(&mut self) {
        if let (Some(cached), Some(query)) = (&self.cached_results, &self.cached_query) {
            if *query == self.search_query {
                let start = self.current_page_index * ITEMS_PER_PAGE;
                let end = (start + ITEMS_PER_PAGE).min(cached.len());
                self.current_files = cached[start..end].to_vec();
                self.total_count = cached.len() as i64;
                return;
            }
        }

        let offset = (self.current_page_index * ITEMS_PER_PAGE) as i64;
        let result = if self.search_query.is_empty() {
            let limit = ITEMS_PER_PAGE as i64;
            self.service.find_files_paginated(offset, limit)
        } else {
            let count = self.service.get_search_count(&self.search_query).unwrap();
            if count <= CACHED_SIZE {
                self.service
                    .search_files_paginated(&self.search_query, 0, count)
            } else {
                self.service.search_files_paginated(
                    &self.search_query,
                    offset,
                    ITEMS_PER_PAGE as i64,
                )
            }
        };

        match result {
            Ok(paginated) => {
                if paginated.total_count <= CACHED_SIZE && !self.search_query.is_empty() {
                    self.cached_results = Some(paginated.files.clone());
                    self.cached_query = Some(self.search_query.clone());

                    let start = self.current_page_index * ITEMS_PER_PAGE;
                    let end = (start + ITEMS_PER_PAGE).min(paginated.files.len());
                    self.current_files = paginated.files[start..end].to_vec();
                    self.total_count = paginated.files.len() as i64;
                } else {
                    self.current_files = paginated.files;
                    self.total_count = paginated.total_count;
                    self.cached_results = None;
                    self.cached_query = None;
                }
            }
            Err(e) => {
                eprintln!("Error loading files: {:?}", e);
                self.current_files = Vec::new();
                self.total_count = 0;
            }
        }
    }

    fn view(&'_ self) -> Element<'_, ReadMessage> {
        let search_section = self.search_section();
        let files = self.files();
        let pagination_section = self.create_pagination_section();

        column![search_section, files, pagination_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn search_section(&'_ self) -> Element<'_, ReadMessage> {
        let search_input = text_input("Search files across all drives...", &self.search_query)
            .on_input(ReadMessage::ContentChanged)
            .on_submit(ReadMessage::SearchSubmit)
            .padding(10)
            .width(Length::Fill);

        let search_button = button(text("Search"))
            .on_press(ReadMessage::SearchSubmit)
            .padding(10);

        let clear_button = button(text("Clear"))
            .on_press(ReadMessage::SearchClear)
            .padding(10)
            .style(button::secondary);

        column![row![search_input, search_button, clear_button].spacing(10)].into()
    }

    fn files(&'_ self) -> Element<'_, ReadMessage> {
        let file_rows: Vec<Element<'_, ReadMessage>> = self
            .current_files
            .iter()
            .map(|file| {
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

        let first_button = button("First")
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

        let prev_button = button("Prev")
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

        let next_button = button("Next")
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

        let last_button = button("Last")
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

        let page_input = text_input("Page #", &self.page_input_value)
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

    pub fn first_page(&mut self) {
        self.current_page_index = 0;
        self.load_current_page();
    }

    pub fn last_page(&mut self) {
        self.current_page_index = self.total_pages() - 1;
        self.load_current_page();
    }

    pub fn previous_page(&mut self) {
        if self.current_page_index > 0 {
            self.current_page_index -= 1;
            self.load_current_page();
        }
    }

    pub fn next_page(&mut self) {
        let total_pages = self.total_pages();
        if self.current_page_index + 1 < total_pages {
            self.current_page_index += 1;
            self.load_current_page();
        }
    }

    pub fn search(&mut self) {
        self.current_page_index = 0;
        self.load_current_page();
    }

    pub fn clear_search(&mut self) {
        self.search_query = String::new();
        self.current_page_index = 0;
        self.load_current_page();
    }

    pub fn go_to_page(&mut self) {
        if let Ok(query) = self.page_input_value.parse::<usize>() {
            if query > 0 && query <= self.total_pages() {
                self.current_page_index = query - 1;
                self.load_current_page();
            }
        }
    }

    fn update(&mut self, message: ReadMessage) {
        match message {
            ReadMessage::PrevPage => self.previous_page(),
            ReadMessage::NextPage => self.next_page(),
            ReadMessage::SearchSubmit => self.search(),
            ReadMessage::SearchClear => self.clear_search(),
            ReadMessage::ContentChanged(content) => self.search_query = content,
            ReadMessage::FirstPage => self.first_page(),
            ReadMessage::LastPage => self.last_page(),
            ReadMessage::PageInputChanged(page_number) => self.page_input_value = page_number,
            ReadMessage::PageInputSubmit => self.go_to_page(),
        }
    }
}

impl WritePage {
    fn new(service: Arc<ListerService>) -> Self {
        Self { service }
    }

    fn view(&'_ self) -> Element<'_, WriteMessage> {
        text("Write Page").into()
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
            button(text("Read").align_x(Alignment::Center))
                .on_press(AppMessage::GoToRead)
                .style(match &self.current_page {
                    Page::Read(_) => button::primary,
                    Page::Write(_) => button::secondary,
                })
                .width(Length::Fill),
            button(text("Write").align_x(Alignment::Center))
                .on_press(AppMessage::GoToWrite)
                .style(match &self.current_page {
                    Page::Read(_) => button::secondary,
                    Page::Write(_) => button::primary,
                })
                .width(Length::Fill)
        ]
        .spacing(10);

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

// #[test]
// fn lister_repository_with_database() {
//     let pool = get_connection_pool("file:memdb1?mode=memory&cache=shared");
//
//     let new_cat = NewFileCategory { name: "Cat Videos" };
//     let another_new_cat = NewFileCategory { name: "Games" };
//     let new_drive = NewDriveEntry {
//         category_id: 2,
//         name: "Windows Drive",
//     };
//     let another_new_drive = NewDriveEntry {
//         category_id: 2,
//         name: "Linux Drive",
//     };
//     let lister_repository = ListerRepository::new(pool);
//
//     let new_category_id = lister_repository.add_category(new_cat).unwrap();
//     let another_new_category_id = lister_repository.add_category(another_new_cat).unwrap();
//
//     let rows = lister_repository.find_all_categories().unwrap();
//
//     let expected = vec![
//         FileCategoryEntity {
//             id: new_category_id,
//             name: "Cat Videos".into(),
//         },
//         FileCategoryEntity {
//             id: another_new_category_id,
//             name: "Games".into(),
//         },
//     ];
//
//     assert_eq!(rows, expected);
//
//     let new_drive_id = lister_repository.add_drive(new_drive).unwrap();
//     let another_new_drive_id = lister_repository.add_drive(another_new_drive).unwrap();
//
//     let rows = lister_repository.find_all_drives_by_category_id(2).unwrap();
//
//     let expected = vec![
//         DriveEntryEntity {
//             id: new_drive_id,
//             category_id: 2,
//             name: "Windows Drive".into(),
//         },
//         DriveEntryEntity {
//             id: another_new_drive_id,
//             category_id: 2,
//             name: "Linux Drive".into(),
//         },
//     ];
//
//     assert_eq!(rows, expected);
// }

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

    fn find_files_paginated(&self, offset: i64, limit: i64) -> RepositoryResult<PaginatedFiles> {
        let mut conn = self.pool.get()?;

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
            .load::<FileWithInfo>(&mut conn)?;

        let files = entities.into_iter().map(|e| e.into()).collect();

        Ok(PaginatedFiles { files, total_count })
    }

    fn search_files_paginated(
        &self,
        search_query: &str,
        offset: i64,
        limit: i64,
    ) -> RepositoryResult<PaginatedFiles> {
        let mut conn = self.pool.get()?;
        let search_pattern = format!("%{}%", search_query.replace(" ", "_"));

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
            .load::<FileWithInfo>(&mut conn)?;

        let files = entities.into_iter().map(|e| e.into()).collect();
        Ok(PaginatedFiles { files, total_count })
    }

    fn get_search_count(&self, search_query: &str) -> RepositoryResult<i64> {
        let mut conn = self.pool.get()?;
        let search_pattern = format!("%{}%", search_query.replace(" ", "_"));
        let count = file_entries::table
            .filter(file_entries::path.like(&search_pattern))
            .count()
            .get_result(&mut conn)?;
        Ok(count)
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

#[derive(Clone)]
struct FileWithInfoModel {
    category_name: String,
    drive_name: String,
    path: String,
    weight: i64,
}

impl FileWithInfoModel {
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

impl From<FileWithInfo> for FileWithInfoModel {
    fn from(value: FileWithInfo) -> Self {
        Self {
            category_name: value.category_name,
            drive_name: value.drive_name,
            path: value.path,
            weight: value.weight,
        }
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

    fn find_files_paginated(&self, offset: i64, limit: i64) -> ServiceResult<PaginatedFiles> {
        let paginated = self.repo.find_files_paginated(offset, limit)?;
        Ok(paginated)
    }

    fn search_files_paginated(
        &self,
        search_query: &str,
        offset: i64,
        limit: i64,
    ) -> ServiceResult<PaginatedFiles> {
        let paginated = self
            .repo
            .search_files_paginated(search_query, offset, limit)?;
        Ok(paginated)
    }

    fn get_search_count(&self, search_query: &str) -> ServiceResult<i64> {
        let count = self.repo.get_search_count(search_query)?;
        Ok(count)
    }
}
