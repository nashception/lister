extern crate libsqlite3_sys;

use crate::schema::{drive_entries, file_categories, file_entries};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as DieselError;
use diesel::ExpressionMethods;
use diesel::{Associations, Identifiable, Insertable, Queryable, RunQueryDsl, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use iced::widget::{button, column, row, text};
use iced::Element;
use std::sync::Arc;

mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
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
    CategorySelected(usize),
    DriveSelected(usize),
}

#[derive(Clone, Debug)]
enum WriteMessage {
    Ok(usize),
}

struct ReadPage {
    service: Arc<ListerService>,

    categories: ServiceResult<Vec<FileCategoryModel>>,
    drives: ServiceResult<Vec<DriveEntryModel>>,

    selected_category: Option<usize>,
    selected_drive: Option<usize>,

    result_files: Vec<FileEntryModel>,
}

struct WritePage {
    service: Arc<ListerService>,
}

impl ReadPage {
    fn new(service: Arc<ListerService>) -> Self {
        let categories = service.list_categories();
        println!("categories: {:?}", categories);
        Self {
            service,
            categories,
            drives: Ok(vec![]),
            selected_category: None,
            selected_drive: None,
            result_files: vec![],
        }
    }

    fn view(&'_ self) -> Element<'_, ReadMessage> {
        let categories = self.categories();
        let drives = self.drives();

        column![row(categories), row(drives)].into()
    }

    fn categories(&'_ self) -> Vec<Element<'_, ReadMessage>> {
        self.categories
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
            .map(|(i, category)| {
                button(text(category.name.clone()))
                    .on_press(ReadMessage::CategorySelected(i))
                    .into()
            })
            .collect()
    }

    fn drives(&'_ self) -> Vec<Element<'_, ReadMessage>> {
        self.drives
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
            .map(|(i, drive)| {
                button(text(drive.name.clone()))
                    .on_press(ReadMessage::DriveSelected(i))
                    .into()
            })
            .collect()
    }

    fn update(&mut self, message: ReadMessage) {
        match message {
            ReadMessage::CategorySelected(index) => {
                if let Some(category) = self.categories.as_ref().unwrap().get(index) {
                    let category_id_usize = category.id as usize;
                    self.selected_category = Some(category_id_usize);
                    self.drives = self
                        .service
                        .find_all_drives_by_category_id(category_id_usize);
                }
            }
            ReadMessage::DriveSelected(index) => self.selected_drive = Some(index),
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
        match &self.current_page {
            Page::Read(page) => page.view().map(AppMessage::Read),
            Page::Write(page) => page.view().map(AppMessage::Write),
        }
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
    let new_files = vec![
        NewFileEntry {
            drive_id: 1,
            path: "Dr House/Season 1/Episode 1.mkv",
            weight: 2000000,
        },
        NewFileEntry {
            drive_id: 1,
            path: "Dr House/Season 1/Episode 2.mkv",
            weight: 2500000,
        },
        NewFileEntry {
            drive_id: 1,
            path: "Dr House/Season 1/Episode 3.mkv",
            weight: 3000000,
        },
        NewFileEntry {
            drive_id: 2,
            path: "Red Dead Redemption Remastered",
            weight: 1500000,
        },
    ];

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

    lister_repository.add_files(new_files).unwrap();

    let files = lister_repository.find_all_files().unwrap();

    assert_eq!(
        files,
        vec![
            FileEntryEntity {
                id: 1,
                drive_id: 1,
                path: String::from("Dr House/Season 1/Episode 1.mkv"),
                weight: 2000000,
            },
            FileEntryEntity {
                id: 2,
                drive_id: 1,
                path: String::from("Dr House/Season 1/Episode 2.mkv"),
                weight: 2500000,
            },
            FileEntryEntity {
                id: 3,
                drive_id: 1,
                path: String::from("Dr House/Season 1/Episode 3.mkv"),
                weight: 3000000,
            },
            FileEntryEntity {
                id: 4,
                drive_id: 2,
                path: String::from("Red Dead Redemption Remastered"),
                weight: 1500000,
            },
        ]
    );

    let files_by_category = lister_repository.find_all_files_by_drive_id(2).unwrap();
    assert_eq!(
        files_by_category,
        vec![FileEntryEntity {
            id: 4,
            drive_id: 2,
            path: String::from("Red Dead Redemption Remastered"),
            weight: 1500000,
        },]
    );
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

    fn find_all_files(&self) -> RepositoryResult<Vec<FileEntryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = file_entries::table.load(&mut conn)?;
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
    path: String,
    weight: i64,
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

    fn list_categories(&self) -> ServiceResult<Vec<FileCategoryModel>> {
        let entities = self.repo.find_all_categories()?;
        Ok(entities.into_iter().map(|e| e.into()).collect())
    }

    fn find_all_drives_by_category_id(
        &self,
        category_id: usize,
    ) -> ServiceResult<Vec<DriveEntryModel>> {
        let entities = self
            .repo
            .find_all_drives_by_category_id(category_id as i32)?;
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

impl From<FileEntryEntity> for FileEntryModel {
    fn from(value: FileEntryEntity) -> Self {
        Self {
            path: value.path,
            weight: value.weight,
        }
    }
}
