use crate::application::file_indexing_service::FileIndexingService;
use crate::domain::model::file_entry::FileEntry;
use crate::infrastructure::filesystem::native_directory_picker::NativeDirectoryPicker;
use crate::tr;
use crate::ui::components::write::indexing::IndexingState;
use crate::ui::messages::write_message::WriteMessage;
use crate::utils::dialogs::{popup_error, popup_error_and_exit};
use iced::widget::{button, column, container, row, text, text_input, Rule};
use iced::{Alignment, Element, Length, Task};
use iced_aw::Spinner;
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
    const fn is_complete(&self) -> bool {
        self.directory.is_some() && !self.category.is_empty() && !self.drive.is_empty()
    }
}

pub struct WritePage {
    indexing_use_case: Arc<FileIndexingService>,
    directory_picker: Arc<NativeDirectoryPicker>,
    state: IndexingState,
    write_data: WriteData,
}

impl WritePage {
    pub fn new(
        indexing_use_case: Arc<FileIndexingService>,
        directory_picker: Arc<NativeDirectoryPicker>,
    ) -> (Self, Task<WriteMessage>) {
        let page = Self {
            indexing_use_case,
            directory_picker,
            state: IndexingState::Ready,
            write_data: WriteData::default(),
        };
        (page, Task::none())
    }

    pub fn title(translations: &HashMap<String, String>) -> String {
        tr!(translations, "write_page_title")
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, WriteMessage> {
        let form_section = self.form_section(translations);
        let action_section = self.action_section(translations);
        let status_section = self.indexing_state(translations);

        column![form_section, action_section, status_section]
            .spacing(20)
            .padding(20)
            .into()
    }

    pub fn update(&mut self, message: WriteMessage) -> Task<WriteMessage> {
        match message {
            WriteMessage::DirectoryPressed { dialog_title } => {
                let picker = self.directory_picker.clone();
                Task::perform(
                    async move { picker.pick_directory(&dialog_title) },
                    WriteMessage::DirectoryChanged,
                )
            }
            WriteMessage::DirectoryChanged(selected_data) => {
                if let Some(data) = selected_data {
                    self.write_data = WriteData {
                        category: data.last_folder_name(),
                        directory: Some(data.directory),
                        drive: data.drive_name,
                        drive_available_space: data.drive_available_space,
                    }
                }
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

        let directory_display = self.write_data.directory.as_ref()
            .map_or_else(|| text(tr!(translations, "no_directory_selected")).style(text::secondary), |dir| 
            text(tr!(translations, "selected_directory", "dir" => &dir.display().to_string()))
                .style(text::success))
        .width(Length::Fill);

        let browse_button = button(text(tr!(translations, "browse_directory")))
            .on_press(WriteMessage::DirectoryPressed {
                dialog_title: tr!(translations, "browse_file_dialog"),
            })
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
        let submit_button = self.submit_button(translations);

        let requirements_text = if self.write_data.is_complete() {
            text("")
        } else {
            text(tr!(translations, "fill_all_fields")).style(text::danger)
        }
        .width(Length::Fill);

        column![Rule::horizontal(1), row![requirements_text, submit_button]]
            .spacing(10)
            .into()
    }

    fn submit_button(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, WriteMessage> {
        if self.state.is_indexing() {
            container(
                Spinner::new()
                    .height(Length::from(45))
                    .width(Length::from(45)),
            )
            .padding(5)
            .into()
        } else {
            let can_submit = self.write_data.is_complete() && self.state == IndexingState::Ready;
            button(text(tr!(translations, "start_indexing")))
                .on_press_maybe(if can_submit {
                    Some(WriteMessage::WriteSubmit)
                } else {
                    None
                })
                .padding(15)
                .style(if can_submit {
                    button::primary
                } else {
                    button::text
                })
                .into()
        }
    }

    fn indexing_state(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, WriteMessage> {
        match self.state {
            IndexingState::Ready => column![],
            IndexingState::CleaningDatabase => column![
                text(tr!(translations, "clean_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "clean_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10),
            IndexingState::Scanning => column![
                text(tr!(translations, "scan_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "scan_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10),
            IndexingState::Saving => column![
                text(tr!(translations, "save_status"))
                    .size(18)
                    .style(text::primary),
                text(tr!(translations, "save_details"))
                    .style(text::secondary)
                    .size(14),
            ]
            .spacing(10),
            IndexingState::Completed { files_indexed } => {
                column![
                    iced::widget::column![
                text(tr!(translations, "done_status"))
                    .size(18)
                    .style(text::success),
                text(tr!(translations, "done_details", "nb_files" => &files_indexed.to_string()))
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
            }
        }
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
                    .unwrap_or_else(|error| popup_error_and_exit(error));
            },
            |()| WriteMessage::DatabaseCleaned,
        )
    }

    fn start_indexing(&mut self) -> Task<WriteMessage> {
        if self.state != IndexingState::CleaningDatabase {
            return Task::none();
        }
        self.state = IndexingState::Scanning;

        let indexing_use_case = self.indexing_use_case.clone();
        self.write_data
            .directory
            .clone()
            .map_or_else(Task::none, |directory| {
                Task::perform(
                    async move {
                        indexing_use_case
                            .scan_directory(&directory)
                            .unwrap_or_else(|error| {
                                popup_error(error);
                                Vec::new()
                            })
                    },
                    WriteMessage::ScanDirectoryFinished,
                )
            })
    }

    fn insert_in_database(&mut self, files: Vec<FileEntry>) -> Task<WriteMessage> {
        if self.state != IndexingState::Scanning {
            return Task::none();
        }
        self.state = IndexingState::Saving;

        let indexing_use_case = self.indexing_use_case.clone();
        let category = self.write_data.category.clone();
        let drive = self.write_data.drive.clone();
        let drive_available_space = self.write_data.drive_available_space;

        Task::perform(
            async move {
                indexing_use_case
                    .insert_in_database(category, drive, drive_available_space, files)
                    .unwrap_or(0)
            },
            WriteMessage::InsertInDatabaseFinished,
        )
    }
}
