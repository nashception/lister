use std::collections::HashMap;
use std::sync::Arc;

use crate::config::constants::{CACHED_SIZE, ITEMS_PER_PAGE};
use crate::domain::entities::file_entry::FileWithMetadata;
use crate::domain::entities::language::Language;
use crate::domain::entities::pagination::PaginatedResult;
use crate::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use crate::tr;
use crate::ui::components::read::cache::Cache;
use crate::ui::components::read::drive_combo_box::DriveComboBox;
use crate::ui::components::read::file_list::FileList;
use crate::ui::components::read::pagination::Pagination;
use crate::ui::components::read::search::Search;
use crate::ui::messages::read_message::ReadMessage;
use crate::utils::dialogs::popup_error;
use iced::keyboard::key::Named;
use iced::widget::{column, row};
use iced::{keyboard, Element, Subscription, Task};

pub struct ReadPage {
    query_use_case: Arc<dyn FileQueryUseCase>,
    drive_combo_box: DriveComboBox,
    search: Search,
    pagination: Pagination,
    file_list: FileList,
    cache: Cache,
    is_cache_warming: bool,
}

impl ReadPage {
    pub fn new(query_use_case: Arc<dyn FileQueryUseCase>) -> (Self, Task<ReadMessage>) {
        let (drive_combo_box, combo_box_task) = DriveComboBox::new(query_use_case.clone());
        let (search, search_task) = Search::new();
        let page = Self {
            query_use_case,
            drive_combo_box,
            search,
            pagination: Pagination::new(ITEMS_PER_PAGE),
            file_list: FileList::new(),
            cache: Cache::new(),
            is_cache_warming: false,
        };
        (page, Task::batch([combo_box_task, search_task]))
    }

    pub fn title(&self, translations: &HashMap<String, String>) -> String {
        tr!(translations, "read_page_title")
    }

    pub fn view(
        &'_ self,
        translations: &HashMap<String, String>,
        language: &Language,
    ) -> Element<'_, ReadMessage> {
        let drive_combo_box = self.drive_combo_box.view(translations);
        let search_section = self.search.view(translations);
        let files = self.file_list.view(language);
        let pagination_section = self.pagination.view(translations);

        column![
            row![drive_combo_box, search_section].spacing(10),
            files,
            pagination_section
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    pub fn update(&mut self, message: ReadMessage) -> Task<ReadMessage> {
        match message {
            ReadMessage::PrevPage => self.previous_page(),
            ReadMessage::NextPage => self.next_page(),
            ReadMessage::FirstPage => self.navigate_to_page(0),
            ReadMessage::LastPage => {
                self.navigate_to_page(self.pagination.total_pages().saturating_sub(1))
            }
            ReadMessage::DrivesFetched(drives) => {
                self.drive_combo_box.drives = drives;
                Task::none()
            }
            ReadMessage::DriveSelected(drive) => {
                self.drive_combo_box.selected_drive = Some(drive);
                self.process_new_search()
            }
            ReadMessage::SearchSubmit => self.process_new_search(),
            ReadMessage::SearchClear => self.clear_search(),
            ReadMessage::ContentChanged(content) => {
                self.search.query = content;
                Task::none()
            }
            ReadMessage::PageInputChanged(page_number) => {
                self.pagination.page_input_value = page_number;
                Task::none()
            }
            ReadMessage::PageInputSubmit => self.process_page_input(),
            ReadMessage::FilesLoaded(result) => self.handle_files_loaded(result),
            ReadMessage::ArrowLeftPressed { shift } => self.handle_left(shift),
            ReadMessage::ArrowRightPressed { shift } => self.handle_right(shift),
            ReadMessage::ArrowUpPressed { shift } => self.file_list.scroll(-30., shift),
            ReadMessage::ArrowDownPressed { shift } => self.file_list.scroll(30., shift),
            ReadMessage::ArrowNavigationReleased => self.load_current_page(),
            ReadMessage::PageUpPressed => self.update(ReadMessage::ArrowUpPressed { shift: true }),
            ReadMessage::PageDownPressed => {
                self.update(ReadMessage::ArrowDownPressed { shift: true })
            }
            ReadMessage::HomePressed => self.file_list.snap_to_top(),
            ReadMessage::EndPressed => self.file_list.snap_to_bottom(),
        }
    }

    pub fn subscription(&self) -> Subscription<ReadMessage> {
        Subscription::batch([
            keyboard::on_key_press(|key, modifiers| {
                let keyboard::Key::Named(key) = key else {
                    return None;
                };
                match (key, modifiers) {
                    (Named::ArrowLeft, _) => Some(ReadMessage::ArrowLeftPressed {
                        shift: modifiers.shift(),
                    }),
                    (Named::ArrowRight, _) => Some(ReadMessage::ArrowRightPressed {
                        shift: modifiers.shift(),
                    }),
                    (Named::ArrowUp, _) => Some(ReadMessage::ArrowUpPressed {
                        shift: modifiers.shift(),
                    }),
                    (Named::ArrowDown, _) => Some(ReadMessage::ArrowDownPressed {
                        shift: modifiers.shift(),
                    }),
                    (Named::PageUp, _) => Some(ReadMessage::PageUpPressed),
                    (Named::PageDown, _) => Some(ReadMessage::PageDownPressed),
                    (Named::Home, _) => Some(ReadMessage::HomePressed),
                    (Named::End, _) => Some(ReadMessage::EndPressed),
                    _ => None,
                }
            }),
            keyboard::on_key_release(|key, _| {
                let keyboard::Key::Named(key) = key else {
                    return None;
                };
                match key {
                    Named::ArrowLeft | Named::ArrowRight => {
                        Some(ReadMessage::ArrowNavigationReleased)
                    }
                    _ => None,
                }
            }),
        ])
    }

    fn load_current_page(&mut self) -> Task<ReadMessage> {
        if let Some(files) = self.cache.get_page(
            &self.drive_combo_box.selected_drive,
            &self.search.query,
            self.pagination.current_page_index,
            ITEMS_PER_PAGE,
        ) {
            self.file_list.set_files(files);
            return self.file_list.snap_to_top();
        }

        if !self
            .cache
            .is_valid_for(&self.drive_combo_box.selected_drive, &self.search.query)
        {
            self.cache.clear();
        }

        let selected_drive = self.drive_combo_box.selected_drive.clone();
        let search_query = if self.search.query.is_empty() {
            None
        } else {
            Some(self.search.query.clone())
        };
        let query_use_case = self.query_use_case.clone();
        let page = self.pagination.current_page_index;
        let ipp = self.pagination.items_per_page;

        Task::perform(
            async move {
                let count = query_use_case
                    .get_search_count(&selected_drive, &search_query)
                    .unwrap_or(0);
                let files = if count <= CACHED_SIZE {
                    query_use_case
                        .search_files(&selected_drive, &search_query, 0, count as usize)
                        .unwrap_or_else(|err| {
                            popup_error(err);
                            vec![]
                        })
                } else {
                    query_use_case
                        .search_files(&selected_drive, &search_query, page, ipp)
                        .unwrap_or_else(|err| {
                            popup_error(err);
                            vec![]
                        })
                };
                PaginatedResult {
                    items: files,
                    total_count: count,
                }
            },
            ReadMessage::FilesLoaded,
        )
    }

    fn previous_page(&mut self) -> Task<ReadMessage> {
        if self.pagination.prev().is_some() {
            self.load_current_page()
        } else {
            Task::none()
        }
    }

    fn next_page(&mut self) -> Task<ReadMessage> {
        if self.pagination.next().is_some() {
            self.load_current_page()
        } else {
            Task::none()
        }
    }

    fn navigate_to_page(&mut self, page_index: usize) -> Task<ReadMessage> {
        if self.pagination.navigate_to(page_index).is_some() {
            self.load_current_page()
        } else {
            Task::none()
        }
    }

    fn process_new_search(&mut self) -> Task<ReadMessage> {
        self.pagination.reset();
        self.load_current_page()
    }

    fn clear_search(&mut self) -> Task<ReadMessage> {
        self.drive_combo_box.selected_drive = None;
        self.search.clear();
        self.cache.clear();
        self.file_list.clear();
        self.pagination.clear();
        Task::none()
    }

    fn process_page_input(&mut self) -> Task<ReadMessage> {
        if let Ok(page) = self.pagination.page_input_value.parse::<usize>() {
            if page > 0 && page <= self.pagination.total_pages() {
                self.navigate_to_page(page - 1)
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    fn handle_files_loaded(&mut self, result: PaginatedResult) -> Task<ReadMessage> {
        self.update_total_count(&result);

        if self.should_warm_cache(&result) {
            self.handle_small_dataset(result)
        } else {
            self.show_page(result.items)
        }
    }

    fn update_total_count(&mut self, result: &PaginatedResult) {
        self.pagination.total_count = result.total_count;
    }

    fn should_warm_cache(&self, result: &PaginatedResult) -> bool {
        result.total_count > 0
            && result.total_count <= CACHED_SIZE
            && self.pagination.current_page_index == 0
    }

    fn handle_small_dataset(&mut self, result: PaginatedResult) -> Task<ReadMessage> {
        // Case A: the result already contains the full dataset -> store & show slice
        if result.items.len() == result.total_count as usize {
            return self.store_full_and_show_page(result.items);
        }

        // Case B: we only received a single page; start warming if not already warming
        if !self.is_cache_warming {
            self.start_cache_warm(result.items)
        } else {
            self.show_page(result.items)
        }
    }

    fn store_full_and_show_page(&mut self, full_items: Vec<FileWithMetadata>) -> Task<ReadMessage> {
        // store full dataset in cache
        self.cache.store(
            self.drive_combo_box.selected_drive.clone(),
            self.search.query.clone(),
            full_items.clone(),
        );

        if let Some(page_files) = self.cache.get_page(
            &self.drive_combo_box.selected_drive,
            &self.search.query,
            self.pagination.current_page_index,
            ITEMS_PER_PAGE,
        ) {
            self.file_list.set_files(page_files);
        } else {
            self.file_list.set_files(Vec::new());
        }

        self.is_cache_warming = false;
        self.file_list.snap_to_top()
    }

    fn start_cache_warm(&mut self, current_page_items: Vec<FileWithMetadata>) -> Task<ReadMessage> {
        // mark warming and show current page immediately
        self.is_cache_warming = true;
        self.file_list.set_files(current_page_items);

        let selected_drive = self.drive_combo_box.selected_drive.clone();
        let search_query = if self.search.query.is_empty() {
            None
        } else {
            Some(self.search.query.clone())
        };
        let query_use_case = self.query_use_case.clone();
        let total = self.pagination.total_count as usize;

        Task::perform(
            async move {
                let files = query_use_case
                    .search_files(&selected_drive, &search_query, 0, total)
                    .unwrap_or_else(|error| {
                        popup_error(error);
                        vec![]
                    });
                PaginatedResult {
                    items: files,
                    total_count: total as i64,
                }
            },
            ReadMessage::FilesLoaded,
        )
    }

    fn show_page(&mut self, items: Vec<FileWithMetadata>) -> Task<ReadMessage> {
        self.file_list.set_files(items);
        self.file_list.snap_to_top()
    }

    fn handle_left(&mut self, shift: bool) -> Task<ReadMessage> {
        if shift {
            self.pagination.first_page();
        } else {
            self.pagination.prev();
        }
        Task::none()
    }

    fn handle_right(&mut self, shift: bool) -> Task<ReadMessage> {
        if shift {
            self.pagination.last_page();
        } else {
            self.pagination.next();
        }
        Task::none()
    }
}
