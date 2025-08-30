use crate::domain::entities::file_entry::FileEntry;
use crate::domain::ports::primary::file_indexing_use_case::FileIndexingUseCase;
use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use crate::tr;
use crate::ui::messages::write_message::WriteMessage;
use crate::ui::utils::translation::tr_impl;
use crate::utils::dialogs::{popup_error, popup_error_and_exit};
use iced::widget::{button, column, row, text, text_input, Rule};
use iced::{Alignment, Element, Length, Task};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
enum IndexingState {
    Ready,
    CleaningDatabase,
    Scanning,
    Saving,
    Completed { files_indexed: usize },
}

pub struct WritePage {
    indexing_use_case: Arc<dyn FileIndexingUseCase>,
    directory_picker: Arc<dyn DirectoryPicker>,
    category: String,
    drive: String,
    directory: Option<PathBuf>,
    state: IndexingState,
    category_input_id: text_input::Id,
}

impl WritePage {
    pub fn new(
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

    pub fn title(&self, translations: &HashMap<String, String>) -> String {
        tr!(translations, "write_page_title")
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, WriteMessage> {
        let form_section = self.form_section(translations);
        let action_section = self.action_section(translations);
        let status_section = self.status_section(translations);

        iced::widget::column![form_section, action_section, status_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    pub fn update(&mut self, message: WriteMessage) -> Task<WriteMessage> {
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
            WriteMessage::WriteSubmit => self.clean_database(),
            WriteMessage::DatabaseCleaned => self.start_indexing(),
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

        iced::widget::column![
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

        iced::widget::column![
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

        iced::widget::column![Rule::horizontal(1), submit_button, requirements_text,]
            .spacing(10)
            .into()
    }

    fn status_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, WriteMessage> {
        match &self.state {
            IndexingState::Ready => iced::widget::column![].into(),
            IndexingState::CleaningDatabase => iced::widget::column![
                text(tr!(translations, "clean_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "clean_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10)
            .into(),
            IndexingState::Scanning => iced::widget::column![
                text(tr!(translations, "scan_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "scan_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10)
            .into(),
            IndexingState::Saving => iced::widget::column![
                text(tr!(translations, "save_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "save_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10)
            .into(),
            IndexingState::Completed { files_indexed } => iced::widget::column![
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

    fn clean_database(&mut self) -> Task<WriteMessage> {
        if self.state != IndexingState::Ready {
            return Task::none();
        }
        self.state = IndexingState::CleaningDatabase;

        let indexing_use_case = self.indexing_use_case.clone();
        let category = self.category.clone();
        let drive = self.drive.clone();

        Task::perform(
            async move {
                indexing_use_case
                    .remove_duplicates(category, drive)
                    .await
                    .unwrap_or_else(|error| popup_error_and_exit(error));
            },
            |_| WriteMessage::DatabaseCleaned,
        )
    }

    fn start_indexing(&mut self) -> Task<WriteMessage> {
        if self.state != IndexingState::CleaningDatabase {
            return Task::none();
        }
        self.state = IndexingState::Scanning;

        let indexing_use_case = self.indexing_use_case.clone();
        if let Some(directory) = self.directory.clone() {
            Task::perform(
                async move {
                    indexing_use_case
                        .scan_directory(directory)
                        .await
                        .unwrap_or_else(|error| {
                            popup_error(error);
                            Vec::new()
                        })
                },
                WriteMessage::ScanDirectoryFinished,
            )
        } else {
            Task::none()
        }
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
