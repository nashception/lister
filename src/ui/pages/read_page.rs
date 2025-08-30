use crate::config::constants::{CACHED_SIZE, ITEMS_PER_PAGE};
use crate::domain::entities::file_entry::FileWithMetadata;
use crate::domain::entities::pagination::PaginatedResult;
use crate::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use crate::tr;
use crate::ui::components::pagination::Pagination;
use crate::ui::messages::read_message::ReadMessage;
use crate::ui::utils::translation::tr_impl;
use crate::utils::dialogs::popup_error;
use humansize::{format_size, DECIMAL};
use iced::keyboard::key::Named;
use iced::widget::{button, column, row, scrollable, text, text_input, Rule};
use iced::{keyboard, Element, Length, Subscription, Task};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ReadPage {
    query_use_case: Arc<dyn FileQueryUseCase>,
    search_query: String,
    current_files: Vec<FileWithMetadata>,
    cached_query: Option<String>,
    cached_results: Option<Vec<FileWithMetadata>>,
    active_task_id: u64,
    scroll_bar_id: scrollable::Id,
    pagination: Pagination,
}

impl ReadPage {
    pub fn new(query_use_case: Arc<dyn FileQueryUseCase>) -> (Self, Task<ReadMessage>) {
        let mut page = Self {
            query_use_case,
            search_query: String::new(),
            current_files: Vec::new(),
            cached_query: None,
            cached_results: None,
            active_task_id: 0,
            scroll_bar_id: scrollable::Id::unique(),
            pagination: Pagination::new(ITEMS_PER_PAGE),
        };
        let task = page.load_current_page();
        (page, task)
    }

    pub fn title(&self, translations: &HashMap<String, String>) -> String {
        tr!(translations, "read_page_title")
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, ReadMessage> {
        let search_section = self.search_section(translations);
        let files = self.files_section();
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
                self.search_query.clear();
                self.process_new_search()
            }
            ReadMessage::ContentChanged(content) => {
                self.search_query = content;
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
            ReadMessage::ArrowUpPressed { shift } => self.scroll(-30., shift),
            ReadMessage::ArrowDownPressed { shift } => self.scroll(30., shift),
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
        // Use cached results if available
        if let (Some(cached), Some(query)) = (&self.cached_results, &self.cached_query) {
            if *query == self.search_query {
                let start = self.pagination.current_page_index * ITEMS_PER_PAGE;
                if start < cached.len() {
                    let end = (start + ITEMS_PER_PAGE).min(cached.len());
                    self.current_files = cached[start..end].to_vec();
                    return Task::none();
                }
            }
        }

        if let Some(query) = &self.cached_query {
            if *query != self.search_query {
                self.cached_results = None;
                self.cached_query = None;
            }
        }

        self.active_task_id += 1;
        let task_id = self.active_task_id;
        let search_query = self.search_query.clone();
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

    fn search_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, ReadMessage> {
        let search_input = text_input(&tr!(translations, "search_placeholder"), &self.search_query)
            .on_input(ReadMessage::ContentChanged)
            .on_submit(ReadMessage::SearchSubmit)
            .padding(10)
            .width(Length::Fill);

        let search_button = button(text(tr!(translations, "search_button")))
            .on_press(ReadMessage::SearchSubmit)
            .padding(10);

        let clear_button = button(text(tr!(translations, "clear_button")))
            .on_press(ReadMessage::SearchClear)
            .padding(10);

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
        self.process_loaded_files(result)
    }

    fn process_loaded_files(&mut self, result: PaginatedResult) -> Task<ReadMessage> {
        if result.total_count <= CACHED_SIZE
            && !self.search_query.is_empty()
            && self.pagination.current_page_index == 0
        {
            self.cached_results = Some(result.items.clone());
            self.cached_query = Some(self.search_query.clone());
            let start = self.pagination.current_page_index * ITEMS_PER_PAGE;
            let end = (start + ITEMS_PER_PAGE).min(result.items.len());
            self.current_files = result.items[start..end].to_vec();
            self.pagination.total_count = result.total_count;
        } else {
            self.current_files = result.items;
            self.pagination.total_count = result.total_count;
        }

        scrollable::snap_to(
            self.scroll_bar_id.clone(),
            scrollable::RelativeOffset::START,
        )
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

    fn scroll(&self, dy: f32, shift: bool) -> Task<ReadMessage> {
        let offset = if shift { dy * 33. } else { dy };
        scrollable::scroll_by(
            self.scroll_bar_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: offset },
        )
    }
}
