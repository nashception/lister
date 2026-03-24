use crate::infrastructure::database::repository::ListerRepository;
use crate::tr;
use crate::ui::components::drive_combo_box::DriveComboBox;
use crate::ui::messages::delete_message::DeleteMessage;
use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;
use crate::utils::dialogs::popup_error;
use iced::widget::{button, column, container, pick_list, row, rule, text};
use iced::{Element, Length, Task};
use std::collections::HashMap;
use std::sync::Arc;

pub struct DeletePage {
    repository: Arc<ListerRepository>,
    drive_combo_box: DriveComboBox,
    categories_per_drive: Vec<String>,
    selected_category: Option<String>,
    is_deleted: bool,
}

impl DeletePage {
    pub fn new(repository: Arc<ListerRepository>) -> (Self, Task<DeleteMessage>) {
        let (drive_combo_box, combo_box_task) = DriveComboBox::new(repository.clone());
        (
            Self {
                repository,
                drive_combo_box,
                categories_per_drive: vec![],
                selected_category: None,
                is_deleted: false,
            },
            combo_box_task.map(DeleteMessage::DriveComboBox),
        )
    }

    pub fn title(translations: &HashMap<String, String>) -> String {
        tr!(translations, "delete_page_title")
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, DeleteMessage> {
        let drive_combo_box = self.drive_combo_box.view(translations);
        let category_combo_box = self.category_combo_box(translations);
        let action_section = self.action_section(translations);

        container(
            column![
                row![
                    drive_combo_box.map(DeleteMessage::DriveComboBox),
                    category_combo_box
                ]
                .spacing(20),
                action_section
            ]
            .padding(20)
            .spacing(20),
        )
        .center_y(Length::Fill)
        .into()
    }

    pub fn update(&mut self, message: DeleteMessage) -> Task<DeleteMessage> {
        match message {
            DeleteMessage::CategoriesFetched(categories) => {
                self.categories_per_drive = categories;
                Task::none()
            }
            DeleteMessage::CategorySelected(category) => {
                self.selected_category = Some(category);
                Task::none()
            }
            DeleteMessage::DriveComboBox(msg) => match msg {
                DriveComboBoxMessage::DrivesFetched(drives) => {
                    self.drive_combo_box.drives = drives;
                    Task::none()
                }
                DriveComboBoxMessage::DriveSelected(drive) => {
                    self.drive_combo_box.selected_drive = Some(drive.clone());
                    self.find_categories_for_drive(drive)
                }
            },
            DeleteMessage::EndDelete => {
                self.is_deleted = true;
                self.drive_combo_box.selected_drive = None;
                self.selected_category = None;
                self.drive_combo_box
                    .find_drives()
                    .map(DeleteMessage::DriveComboBox)
            }
            DeleteMessage::StartDelete => {
                self.is_deleted = false;
                self.delete()
            }
        }
    }

    fn category_combo_box(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, DeleteMessage> {
        pick_list(
            self.categories_per_drive.clone(),
            self.selected_category.clone(),
            DeleteMessage::CategorySelected,
        )
        .placeholder(tr!(translations, "select_category_placeholder"))
        .into()
    }

    fn action_section(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, DeleteMessage> {
        let submit_button = self.submit_button(translations);

        let requirements_text = if self.can_submit() {
            if self.is_deleted {
                text(tr!(translations, "delete_completed"))
            } else {
                text("")
            }
        } else {
            text(tr!(translations, "delete_select_drive")).style(text::danger)
        }
        .width(Length::Fill);

        column![rule::horizontal(1), row![requirements_text, submit_button]]
            .spacing(20)
            .into()
    }

    fn submit_button(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, DeleteMessage> {
        let can_submit = self.can_submit();
        button(text(tr!(translations, "start_deleting")))
            .on_press_maybe(if can_submit {
                Some(DeleteMessage::StartDelete)
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

    const fn can_submit(&self) -> bool {
        self.drive_combo_box.selected_drive.is_some()
    }

    fn find_categories_for_drive(&self, drive: String) -> Task<DeleteMessage> {
        let repository = self.repository.clone();
        Task::perform(
            async move {
                repository
                    .find_all_category_names_for_drive(&drive)
                    .unwrap_or_else(|err| {
                        popup_error(err);
                        vec![]
                    })
            },
            DeleteMessage::CategoriesFetched,
        )
    }

    fn delete(&self) -> Task<DeleteMessage> {
        let command_repository = self.repository.clone();
        let drive = self.drive_combo_box.selected_drive.clone().unwrap();
        let category = self.selected_category.clone();
        Task::perform(
            async move {
                command_repository
                    .delete(&drive, category.as_deref())
                    .unwrap_or_else(popup_error);
            },
            |()| DeleteMessage::EndDelete,
        )
    }
}
