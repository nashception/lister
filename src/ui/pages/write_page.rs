use crate::domain::entities::file_entry::FileEntry;
use crate::domain::ports::primary::file_indexing_use_case::FileIndexingUseCase;
use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use crate::tr;
use crate::ui::components::write::indexing::{indexing_spinner, indexing_state, IndexingState};
use crate::ui::messages::write_message::WriteMessage;
use crate::ui::utils::translation::tr_impl;
use crate::utils::dialogs::{popup_error, popup_error_and_exit};
use iced::widget::{button, column, row, text, text_input, Rule};
use iced::{Alignment, Element, Length, Task};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Default)]
struct WriteData {
    category: String,
    directory: Option<PathBuf>,
    drive: String,
    drive_available_space: u64,
}

impl WriteData {
    fn is_complete(&self) -> bool {
        self.directory.is_some() && !self.category.is_empty() && !self.drive.is_empty()
    }
}

pub struct WritePage {
    indexing_use_case: Arc<dyn FileIndexingUseCase>,
    directory_picker: Arc<dyn DirectoryPicker>,
    state: IndexingState,
    write_data: WriteData,
}

impl WritePage {
    pub fn new(
        indexing_use_case: Arc<dyn FileIndexingUseCase>,
        directory_picker: Arc<dyn DirectoryPicker>,
    ) -> (Self, Task<WriteMessage>) {
        let page = Self {
            indexing_use_case,
            directory_picker,
            state: IndexingState::Ready,
            write_data: Default::default(),
        };
        (page, Task::none())
    }

    pub fn title(&self, translations: &HashMap<String, String>) -> String {
        tr!(translations, "write_page_title")
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, WriteMessage> {
        let form_section = self.form_section(translations);
        let action_section = self.action_section(translations);
        let spinner = indexing_spinner(&self.state);
        let status_section = indexing_state(&self.state, translations);

        column![
            form_section,
            action_section,
            row![spinner, status_section].spacing(10)
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    pub fn update(&mut self, message: WriteMessage) -> Task<WriteMessage> {
        match message {
            WriteMessage::DirectoryPressed => {
                let picker = self.directory_picker.clone();
                Task::perform(
                    async move { picker.pick_directory().await },
                    WriteMessage::DirectoryChanged,
                )
            }
            WriteMessage::DirectoryChanged(selected_data) => {
                if let Some(data) = selected_data {
                    self.write_data = WriteData {
                        category: data.file_name(),
                        directory: Some(data.directory),
                        drive: data.drive_name,
                        drive_available_space: data.drive_available_space,
                    }
                };
                Task::none()
            }
            WriteMessage::CategoryChanged(value) => {
                self.write_data.category = value;
                Task::none()
            }
            WriteMessage::DiskChanged(value) => {
                self.write_data.drive = value;
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
                self.state = IndexingState::Ready;
                Task::none()
            }
        }
    }

    fn form_section(&'_ self, translations: &HashMap<String, String>) -> Element<'_, WriteMessage> {
        let directory_section = self.directory_section(translations);

        let category_input = text_input(
            &tr!(translations, "category_placeholder"),
            &self.write_data.category,
        )
        .on_input(WriteMessage::CategoryChanged)
        .padding(10)
        .width(Length::Fill);

        let drive_input = text_input(
            &tr!(translations, "drive_placeholder"),
            &self.write_data.drive,
        )
        .on_input(WriteMessage::DiskChanged)
        .padding(10)
        .width(Length::Fill);

        column![
            text(tr!(translations, "file_indexing_setup"))
                .size(24)
                .style(text::primary),
            Rule::horizontal(1),
            directory_section,
            column![
                text(tr!(translations, "category_label")).size(16),
                category_input,
            ]
            .spacing(5),
            column![text(tr!(translations, "drive_label")).size(16), drive_input,].spacing(5),
        ]
        .spacing(15)
        .into()
    }

    fn directory_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, WriteMessage> {
        let directory_label = text(tr!(translations, "directory_label")).size(16);

        let directory_display = if let Some(dir) = &self.write_data.directory {
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
        let can_submit = self.write_data.is_complete() && self.state == IndexingState::Ready;

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

        let requirements_text = if !(!self.write_data.is_complete()) {
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

    fn clean_database(&mut self) -> Task<WriteMessage> {
        if self.state != IndexingState::Ready {
            return Task::none();
        }
        self.state = IndexingState::CleaningDatabase;

        let indexing_use_case = self.indexing_use_case.clone();
        let category = self.write_data.category.clone();
        let drive = self.write_data.drive.clone();

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
        if let Some(directory) = self.write_data.directory.clone() {
            Task::perform(
                async move {
                    indexing_use_case
                        .scan_directory(&directory)
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
        let category = self.write_data.category.clone();
        let drive = self.write_data.drive.clone();
        let drive_available_space = self.write_data.drive_available_space.clone();

        Task::perform(
            async move {
                indexing_use_case
                    .insert_in_database(category, drive, drive_available_space as i64, files)
                    .await
                    .unwrap_or(0)
            },
            WriteMessage::InsertInDatabaseFinished,
        )
    }
}
