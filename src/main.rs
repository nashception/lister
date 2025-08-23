extern crate libsqlite3_sys;

use crate::schema::{drive_entries, file_categories, file_entries};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as DieselError;
use diesel::{Associations, Identifiable, Insertable, Queryable, RunQueryDsl, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use humansize::{format_size, DECIMAL};
use iced::widget::scrollable::RelativeOffset;
use iced::widget::{button, column, row, scrollable, text, text_input, Rule};
use iced::window::{icon, Icon, Settings};
use iced::{Alignment, Element, Length, Task};
use rfd::AsyncFileDialog;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use tokio::runtime::Runtime;
use walkdir::WalkDir;

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

    repository
        .add_category(NewFileCategory {
            name: "Series".to_string(),
        })
        .expect("add_category failed");
    repository
        .add_drive(NewDriveEntry {
            category_id: 1,
            name: "Windows Drive".to_string(),
        })
        .expect("add_drive failed");

    let files: Vec<NewFileEntry> = backing
        .iter()
        .enumerate()
        .map(|(i, s)| NewFileEntry {
            drive_id: 1,
            path: String::from(s),
            weight: 200_000 + (i as i64) * 42,
        })
        .collect();
    repository.add_files(files).expect("add_files failed");
}

static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .expect("failed to build Tokio runtime")
});

fn main() -> iced::Result {
    iced::application("Lister", ListerApp::update, ListerApp::view)
        .window(window())
        .run_with(|| ListerApp::new())
}

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
    FilesLoaded((u64, PaginatedFiles)),
}

#[derive(Clone, Debug)]
enum WriteMessage {
    CategoryChanged(String),
    CategorySubmit,
    DriveChanged(String),
    DriveSubmit,
    DirectoryPressed,
    DirectoryChanged(Option<PathBuf>),
    WriteSubmit,
    WalkFinished(Vec<FileEntryModel>),
    InsertFinished,
    ResetForm,
}

#[derive(Clone, Debug)]
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

    active_task_id: u64,
    search_input_id: text_input::Id,
    scroll_bar_id: scrollable::Id,
}

struct WritePage {
    service: Arc<ListerService>,
    category: String,
    drive: String,
    directory: Option<PathBuf>,

    is_walking_directory: bool,
    is_inserting_in_database: bool,
    is_finished: bool,
    category_input_id: text_input::Id,
    drive_input_id: text_input::Id,
}

impl ReadPage {
    fn new(service: Arc<ListerService>) -> (Self, Task<ReadMessage>) {
        let mut page = Self {
            service,
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

    fn load_current_page(&mut self) -> Task<ReadMessage> {
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
        let service = self.service.clone();
        let offset = (self.current_page_index * ITEMS_PER_PAGE) as i64;

        Task::perform(
            async move {
                let result: ServiceResult<PaginatedFiles> = if search_query.is_empty() {
                    service
                        .find_files_paginated(offset, ITEMS_PER_PAGE as i64)
                        .await
                } else {
                    match service.get_search_count(&search_query).await {
                        Ok(count) => {
                            if count <= CACHED_SIZE {
                                service
                                    .search_files_paginated(&search_query, 0, count)
                                    .await
                            } else {
                                service
                                    .search_files_paginated(
                                        &search_query,
                                        offset,
                                        ITEMS_PER_PAGE as i64,
                                    )
                                    .await
                            }
                        }
                        Err(e) => Err(e),
                    }
                };
                (task_id, result)
            },
            |(finished_task_id, result)| {
                ReadMessage::FilesLoaded((finished_task_id, result.unwrap()))
            },
        )
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
            .id(self.search_input_id.clone())
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
            scrollable(column(file_rows))
                .id(self.scroll_bar_id.clone())
                .height(Length::Fill),
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

    fn update(&mut self, message: ReadMessage) -> Task<ReadMessage> {
        match message {
            ReadMessage::PrevPage => self.previous_page(),
            ReadMessage::NextPage => self.next_page(),
            ReadMessage::SearchSubmit => self.search(),
            ReadMessage::SearchClear => self.clear_search(),
            ReadMessage::ContentChanged(content) => self.content_changed(content),
            ReadMessage::FirstPage => self.first_page(),
            ReadMessage::LastPage => self.last_page(),
            ReadMessage::PageInputChanged(page_number) => self.page_input_changed(page_number),
            ReadMessage::PageInputSubmit => self.go_to_page(),
            ReadMessage::FilesLoaded((task_id, result)) => self.files_loaded(task_id, result),
        }
    }

    fn previous_page(&mut self) -> Task<ReadMessage> {
        if self.current_page_index > 0 {
            self.current_page_index -= 1;
            return self.load_current_page();
        }
        Task::none()
    }

    fn next_page(&mut self) -> Task<ReadMessage> {
        let total_pages = self.total_pages();
        if self.current_page_index + 1 < total_pages {
            self.current_page_index += 1;
            return self.load_current_page();
        }
        Task::none()
    }

    fn search(&mut self) -> Task<ReadMessage> {
        self.current_page_index = 0;
        self.load_current_page()
    }

    fn clear_search(&mut self) -> Task<ReadMessage> {
        self.search_query = String::new();
        self.current_page_index = 0;
        self.load_current_page()
    }

    fn content_changed(&mut self, content: String) -> Task<ReadMessage> {
        self.search_query = content;
        Task::none()
    }

    fn first_page(&mut self) -> Task<ReadMessage> {
        self.current_page_index = 0;
        self.load_current_page()
    }

    fn last_page(&mut self) -> Task<ReadMessage> {
        self.current_page_index = self.total_pages() - 1;
        self.load_current_page()
    }

    fn page_input_changed(&mut self, page_number: String) -> Task<ReadMessage> {
        self.page_input_value = page_number;
        Task::none()
    }

    fn go_to_page(&mut self) -> Task<ReadMessage> {
        if let Ok(query) = self.page_input_value.parse::<usize>() {
            if query > 0 && query <= self.total_pages() {
                self.current_page_index = query - 1;
                return self.load_current_page();
            }
        }
        Task::none()
    }

    fn files_loaded(&mut self, task_id: u64, result: PaginatedFiles) -> Task<ReadMessage> {
        let task = if task_id == self.active_task_id {
            self.process_loaded_files(result)
        } else {
            Task::none()
        };
        task.chain(text_input::focus(self.search_input_id.clone()))
    }

    fn process_loaded_files(&mut self, result: PaginatedFiles) -> Task<ReadMessage> {
        if result.total_count <= CACHED_SIZE && !self.search_query.is_empty() {
            self.cached_results = Some(result.files.clone());
            self.cached_query = Some(self.search_query.clone());
            let start = self.current_page_index * ITEMS_PER_PAGE;
            let end = (start + ITEMS_PER_PAGE).min(result.files.len());
            self.current_files = result.files[start..end].to_vec();
            self.total_count = result.files.len() as i64;
        } else {
            self.current_files = result.files;
            self.total_count = result.total_count;
            self.cached_results = None;
            self.cached_query = None;
        }
        scrollable::snap_to(self.scroll_bar_id.clone(), RelativeOffset::START)
    }
}

impl WritePage {
    fn new(service: Arc<ListerService>) -> (Self, Task<WriteMessage>) {
        let category_input_id = text_input::Id::unique();
        let page = Self {
            service,
            category: "".to_string(),
            drive: "".to_string(),
            directory: None,
            is_walking_directory: false,
            is_inserting_in_database: false,
            is_finished: false,
            category_input_id: category_input_id.clone(),
            drive_input_id: text_input::Id::unique(),
        };
        (page, text_input::focus(category_input_id))
    }

    fn view(&'_ self) -> Element<'_, WriteMessage> {
        let form_section = self.create_form_section();
        let action_section = self.create_action_section();
        let status_section = self.create_status_section();

        column![form_section, action_section, status_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    fn create_form_section(&'_ self) -> Element<'_, WriteMessage> {
        let category_input = text_input(
            "Enter category name (e.g., Movies, Documents, Music)",
            &self.category,
        )
        .on_input(WriteMessage::CategoryChanged)
        .id(self.category_input_id.clone())
        .on_submit(WriteMessage::CategorySubmit)
        .padding(10)
        .width(Length::Fill);

        let drive_input = text_input(
            "Enter drive name (e.g., External HDD, C: Drive)",
            &self.drive,
        )
        .on_input(WriteMessage::DriveChanged)
        .id(self.drive_input_id.clone())
        .on_submit(WriteMessage::DriveSubmit)
        .padding(10)
        .width(Length::Fill);

        let directory_section = self.create_directory_section();

        column![
            text("File Indexing Setup").size(24).style(text::primary),
            Rule::horizontal(1),
            column![text("Category").size(16), category_input,].spacing(5),
            column![text("Drive Name").size(16), drive_input,].spacing(5),
            directory_section,
        ]
        .spacing(15)
        .into()
    }

    fn create_directory_section(&'_ self) -> Element<'_, WriteMessage> {
        let directory_label = text("Directory").size(16);

        let directory_display = if let Some(dir) = &self.directory {
            text(format!("Selected: {}", dir.display())).style(text::success)
        } else {
            text("No directory selected").style(text::secondary)
        };

        let browse_button = button(text("Browse Directory"))
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

    fn create_action_section(&'_ self) -> Element<'_, WriteMessage> {
        let can_submit = !self.category.is_empty()
            && !self.drive.is_empty()
            && self.directory.is_some()
            && !self.is_walking_directory
            && !self.is_inserting_in_database;

        let submit_button = button(text("Start Indexing"))
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

        let requirements_text =
            if !can_submit && !self.is_walking_directory && !self.is_inserting_in_database {
                text("Please fill in all fields to start indexing")
                    .style(text::secondary)
                    .size(12)
            } else {
                text("")
            };

        column![Rule::horizontal(1), submit_button, requirements_text,]
            .spacing(10)
            .into()
    }

    fn create_status_section(&'_ self) -> Element<'_, WriteMessage> {
        if !self.is_walking_directory && !self.is_inserting_in_database && !self.is_finished {
            return column![].into();
        }

        let status_content = if self.is_walking_directory {
            column![
                text("[SCAN] Scanning Directory")
                    .size(18)
                    .style(text::primary),
                text("Finding files to index... This may take a while for large directories.")
                    .style(text::secondary)
                    .size(14),
            ]
        } else if self.is_inserting_in_database {
            column![
                text("[SAVE] Inserting Data").size(18).style(text::primary),
                text("Adding files to database... Please wait.")
                    .style(text::secondary)
                    .size(14),
            ]
        } else if self.is_finished {
            column![
                text("[DONE] Indexing Complete")
                    .size(18)
                    .style(text::success),
                text("Files have been successfully indexed and added to the database.")
                    .style(text::success)
                    .size(14),
                button(text("Start New Indexing"))
                    .on_press(WriteMessage::ResetForm)
                    .padding(10)
                    .style(button::secondary),
            ]
        } else {
            column![]
        };

        column![Rule::horizontal(1), status_content.spacing(10),]
            .spacing(15)
            .into()
    }

    fn update(&mut self, message: WriteMessage) -> Task<WriteMessage> {
        match message {
            WriteMessage::CategoryChanged(result) => self.category_changed(result),
            WriteMessage::CategorySubmit => self.category_submit(),
            WriteMessage::DriveChanged(result) => self.drive_changed(result),
            WriteMessage::DriveSubmit => self.drive_submit(),
            WriteMessage::DirectoryPressed => Self::choose_directory(),
            WriteMessage::DirectoryChanged(result) => self.directory_changed(result),
            WriteMessage::WriteSubmit => self.walk_directory(),
            WriteMessage::WalkFinished(files) => self.insert_in_database(files),
            WriteMessage::InsertFinished => self.insert_finished(),
            WriteMessage::ResetForm => self.reset_form(),
        }
    }

    fn category_changed(&mut self, result: String) -> Task<WriteMessage> {
        self.category = result;
        Task::none()
    }

    fn category_submit(&mut self) -> Task<WriteMessage> {
        text_input::focus(self.drive_input_id.clone())
    }

    fn drive_changed(&mut self, result: String) -> Task<WriteMessage> {
        self.drive = result;
        Task::none()
    }

    fn drive_submit(&mut self) -> Task<WriteMessage> {
        text_input::focus(self.category_input_id.clone())
    }

    fn choose_directory() -> Task<WriteMessage> {
        Task::perform(
            async {
                AsyncFileDialog::new()
                    .set_title("Select Directory to Index")
                    .pick_folder()
                    .await
                    .map(|handle| handle.path().to_path_buf())
            },
            WriteMessage::DirectoryChanged,
        )
    }

    fn directory_changed(&mut self, result: Option<PathBuf>) -> Task<WriteMessage> {
        self.directory = result;
        Task::none()
    }

    fn walk_directory(&mut self) -> Task<WriteMessage> {
        self.is_walking_directory = true;
        if let Some(directory) = self.directory.clone() {
            return Task::perform(
                async { walk_directory(directory).await },
                WriteMessage::WalkFinished,
            );
        }
        Task::none()
    }

    fn insert_in_database(&mut self, files: Vec<FileEntryModel>) -> Task<WriteMessage> {
        self.is_walking_directory = false;
        self.is_inserting_in_database = true;
        let category = self.category.clone();
        let drive = self.drive.clone();
        let service = self.service.clone();
        Task::perform(
            async move {
                let category_id = service
                    .add_category(NewFileCategory { name: category })
                    .await?;
                let drive_id = service
                    .add_drive(NewDriveEntry {
                        category_id,
                        name: drive,
                    })
                    .await?;
                service.add_files(drive_id, files).await?;
                Ok::<(), ServiceError>(())
            },
            |_| WriteMessage::InsertFinished,
        )
    }

    fn insert_finished(&mut self) -> Task<WriteMessage> {
        self.is_inserting_in_database = false;
        self.is_finished = true;
        Task::none()
    }

    fn reset_form(&mut self) -> Task<WriteMessage> {
        self.category = String::new();
        self.drive = String::new();
        self.directory = None;
        self.is_walking_directory = false;
        self.is_inserting_in_database = false;
        self.is_finished = false;
        text_input::focus(self.category_input_id.clone())
    }
}

async fn walk_directory(path: PathBuf) -> Vec<FileEntryModel> {
    TOKIO_RUNTIME
        .handle()
        .spawn_blocking(move || {
            WalkDir::new(&path)
                .sort_by_file_name()
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| file_info(&path, e.path()))
                .collect()
        })
        .await
        .unwrap()
}

fn file_info(chosen_directory_path: &PathBuf, absolute_file_path: &Path) -> FileEntryModel {
    FileEntryModel {
        path: file_path(chosen_directory_path, absolute_file_path),
        weight: weight(absolute_file_path),
    }
}

fn file_path(chosen_directory_path: &PathBuf, absolute_file_path: &Path) -> String {
    absolute_file_path
        .strip_prefix(chosen_directory_path)
        .expect("File not under chosen directory")
        .to_path_buf()
        .to_string_lossy()
        .into_owned()
}

pub fn weight(path: &Path) -> i64 {
    fs::metadata(path)
        .expect("Cannot access file metadata")
        .len() as i64
}

struct ListerApp {
    service: Arc<ListerService>,
    current_page: Page,
}

fn init_back_end() -> Arc<ListerService> {
    let pool = get_connection_pool("app.db");
    Arc::new(ListerService::new(ListerRepository::new(pool)))
}

impl ListerApp {
    fn new() -> (Self, Task<AppMessage>) {
        let service = init_back_end();
        let (page, task) = ReadPage::new(service.clone());
        (
            Self {
                service,
                current_page: Page::Read(page),
            },
            task.map(AppMessage::Read),
        )
    }

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

    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match &mut self.current_page {
            Page::Read(page) => match message {
                AppMessage::GoToWrite => self.init_write_page(),
                AppMessage::Read(msg) => page.update(msg).map(AppMessage::Read),
                _ => Task::none(),
            },
            Page::Write(page) => match message {
                AppMessage::GoToRead => self.init_read_page(),
                AppMessage::Write(msg) => page.update(msg).map(AppMessage::Write),
                _ => Task::none(),
            },
        }
    }

    fn init_write_page(&mut self) -> Task<AppMessage> {
        let (write_page, task) = WritePage::new(self.service.clone());
        self.current_page = Page::Write(write_page);
        task.map(AppMessage::Write)
    }

    fn init_read_page(&mut self) -> Task<AppMessage> {
        let (page, task) = ReadPage::new(self.service.clone());
        self.current_page = Page::Read(page);
        task.map(AppMessage::Read)
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
    fn add_category(&self, category: NewFileCategory) -> RepositoryResult<i32> {
        let mut conn = self.pool.get()?;
        let id = diesel::insert_into(file_categories::table)
            .values(category)
            .returning(file_categories::id)
            .get_result(&mut conn)?;
        Ok(id)
    }

    fn add_drive(&self, drive: NewDriveEntry) -> RepositoryResult<i32> {
        let mut conn = self.pool.get()?;
        let id = diesel::insert_into(drive_entries::table)
            .values(drive)
            .returning(drive_entries::id)
            .get_result(&mut conn)?;
        Ok(id)
    }

    fn add_files(&self, files: Vec<NewFileEntry>) -> RepositoryResult<()> {
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
struct NewFileCategory {
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = drive_entries)]
struct NewDriveEntry {
    category_id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = file_entries)]
struct NewFileEntry {
    drive_id: i32,
    path: String,
    weight: i64,
}

#[derive(Clone, Debug)]
struct FileEntryModel {
    path: String,
    weight: i64,
}

#[derive(Clone, Debug)]
struct FileWithInfoModel {
    category_name: String,
    drive_name: String,
    path: String,
    weight: i64,
}

impl FileEntryModel {
    fn into_new_file_entry(self, drive_id: i32) -> NewFileEntry {
        NewFileEntry {
            drive_id,
            path: self.path,
            weight: self.weight,
        }
    }
}

impl FileWithInfoModel {
    fn parent_dir(&self) -> String {
        Path::new(&self.path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "".to_string())
    }

    fn file_name(&self) -> String {
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

struct ListerService {
    repo: ListerRepository,
}

impl ListerService {
    fn new(repo: ListerRepository) -> Self {
        ListerService { repo }
    }

    async fn add_category(&self, category_name: NewFileCategory) -> ServiceResult<i32> {
        let repo = self.repo.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || repo.add_category(category_name))
            .await
            .unwrap()
            .map_err(ServiceError::Repo)
    }

    async fn add_drive(&self, drive: NewDriveEntry) -> ServiceResult<i32> {
        let repo = self.repo.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || repo.add_drive(drive))
            .await
            .unwrap()
            .map_err(ServiceError::Repo)
    }

    async fn add_files(&self, drive_id: i32, files: Vec<FileEntryModel>) -> ServiceResult<()> {
        let repo = self.repo.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || {
                repo.add_files(
                    files
                        .into_iter()
                        .map(|f| f.into_new_file_entry(drive_id))
                        .collect(),
                )
            })
            .await
            .unwrap()
            .map_err(ServiceError::Repo)
    }

    async fn find_files_paginated(&self, offset: i64, limit: i64) -> ServiceResult<PaginatedFiles> {
        let repo = self.repo.clone();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || repo.find_files_paginated(offset, limit))
            .await
            .unwrap()
            .map_err(ServiceError::Repo)
    }

    async fn search_files_paginated(
        &self,
        search_query: &str,
        offset: i64,
        limit: i64,
    ) -> ServiceResult<PaginatedFiles> {
        let repo = self.repo.clone();
        let query = search_query.to_string();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || repo.search_files_paginated(&query, offset, limit))
            .await
            .unwrap()
            .map_err(ServiceError::Repo)
    }

    async fn get_search_count(&self, search_query: &str) -> ServiceResult<i64> {
        let repo = self.repo.clone();
        let query = search_query.to_string();
        TOKIO_RUNTIME
            .handle()
            .spawn_blocking(move || repo.get_search_count(&query))
            .await
            .unwrap()
            .map_err(ServiceError::Repo)
    }
}
