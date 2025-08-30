use crate::ui::utils::translation::tr_impl;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::pagination::PaginatedResult;
use crate::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use crate::tr;
use crate::ui::components::cache::Cache;
use crate::ui::components::file_list::FileList;
use crate::ui::components::pagination::Pagination;
use crate::ui::components::search::Search;
use crate::ui::messages::read_message::ReadMessage;
use crate::utils::dialogs::popup_error;
use iced::keyboard::key::Named;
use iced::widget::column;
use iced::{keyboard, Element, Subscription, Task};
use crate::config::constants::{CACHED_SIZE, ITEMS_PER_PAGE};

pub struct ReadPage {
    query_use_case: Arc<dyn FileQueryUseCase>,
    search: Search,
    pagination: Pagination,
    file_list: FileList,
    cache: Cache,
    active_task_id: u64,
}

impl ReadPage {
    pub fn new(query_use_case: Arc<dyn FileQueryUseCase>) -> (Self, Task<ReadMessage>) {
        let mut page = Self {
            query_use_case,
            search: Search::new(),
            pagination: Pagination::new(ITEMS_PER_PAGE),
            file_list: FileList::new(),
            cache: Cache::new(),
            active_task_id: 0,
        };
        let task = page.load_current_page();
        (page, task)
    }

    pub fn title(&self, translations: &HashMap<String, String>) -> String {
        tr!(translations, "read_page_title")
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, ReadMessage> {
        let search_section = self.search.view(translations);
        let files = self.file_list.view();
        let pagination_section = self.pagination.view(translations);

        column![search_section, files, pagination_section]
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
            ReadMessage::SearchSubmit => self.process_new_search(),
            ReadMessage::SearchClear => {
                self.search.clear();
                self.process_new_search()
            }
            ReadMessage::ContentChanged(content) => {
                self.search.query = content;
                Task::none()
            }
            ReadMessage::PageInputChanged(page_number) => {
                self.pagination.page_input_value = page_number;
                Task::none()
            }
            ReadMessage::PageInputSubmit => self.process_page_input(),
            ReadMessage::FilesLoaded { task_id, result } => {
                self.handle_files_loaded(task_id, result)
            }
            ReadMessage::ArrowLeftPressed { shift } => self.handle_left(shift),
            ReadMessage::ArrowRightPressed { shift } => self.handle_right(shift),
            ReadMessage::ArrowUpPressed { shift } => self.file_list.scroll(-30., shift),
            ReadMessage::ArrowDownPressed { shift } => self.file_list.scroll(30., shift),
        }
    }

    pub fn subscription(&self) -> Subscription<ReadMessage> {
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
                _ => None,
            }
        })
    }

    fn load_current_page(&mut self) -> Task<ReadMessage> {
        if let Some(files) = self.cache.get_page(
            &self.search.query,
            self.pagination.current_page_index,
            ITEMS_PER_PAGE,
        ) {
            self.file_list.set_files(files);
            return Task::none();
        }

        if !self.cache.is_valid_for(&self.search.query) {
            self.cache.clear();
        }

        self.active_task_id += 1;
        let task_id = self.active_task_id;
        let search_query = self.search.query.clone();
        let query_use_case = self.query_use_case.clone();
        let page = self.pagination.current_page_index;

        Task::perform(
            async move {
                let result = if search_query.is_empty() {
                    query_use_case.list_files(page, ITEMS_PER_PAGE).await
                } else {
                    query_use_case
                        .search_files(&search_query, page, ITEMS_PER_PAGE)
                        .await
                }
                .unwrap_or_else(|error| {
                    popup_error(error);
                    PaginatedResult {
                        items: vec![],
                        total_count: 0,
                    }
                });
                (task_id, result)
            },
            |(task_id, result)| ReadMessage::FilesLoaded { task_id, result },
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

    fn handle_files_loaded(&mut self, task_id: u64, result: PaginatedResult) -> Task<ReadMessage> {
        if task_id != self.active_task_id {
            return Task::none();
        }

        if result.total_count <= CACHED_SIZE
            && !self.search.query.is_empty()
            && self.pagination.current_page_index == 0
        {
            self.cache
                .store(self.search.query.clone(), result.items.clone());
        }

        self.file_list.set_files(result.items);
        self.pagination.total_count = result.total_count;

        self.file_list.snap_to_top()
    }

    fn handle_left(&mut self, shift: bool) -> Task<ReadMessage> {
        if self.pagination.current_page_index > 0 {
            self.update(if shift {
                ReadMessage::FirstPage
            } else {
                ReadMessage::PrevPage
            })
        } else {
            Task::none()
        }
    }

    fn handle_right(&mut self, shift: bool) -> Task<ReadMessage> {
        if self.pagination.current_page_index < self.pagination.total_pages().saturating_sub(1) {
            self.update(if shift {
                ReadMessage::LastPage
            } else {
                ReadMessage::NextPage
            })
        } else {
            Task::none()
        }
    }
}
