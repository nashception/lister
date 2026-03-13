use crate::application::delete_service::DeleteService;
use crate::application::file_query_service::FileQueryService;
use crate::tr;
use crate::ui::components::delete::category_combo_box::CategoryComboBox;
use crate::ui::components::drive_combo_box::DriveComboBox;
use crate::ui::messages::category_combo_box::CategoryComboBoxMessage;
use crate::ui::messages::delete_message::DeleteMessage;
use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;
use crate::utils::dialogs::popup_error;
use iced::widget::{button, column, container, row, rule, text};
use iced::{Element, Length, Task};
use std::collections::HashMap;
use std::sync::Arc;

pub struct DeletePage {
    delete_use_case: Arc<DeleteService>,
    query_use_case: Arc<FileQueryService>,
    category_combo_box: CategoryComboBox,
    drive_combo_box: DriveComboBox,
    is_deleted: bool,
}

impl DeletePage {
    pub fn new(
        delete_use_case: Arc<DeleteService>,
        query_use_case: Arc<FileQueryService>,
    ) -> (Self, Task<DeleteMessage>) {
        let (drive_combo_box, combo_box_task) = DriveComboBox::new(query_use_case.clone());
        (
            Self {
                delete_use_case,
                query_use_case,
                category_combo_box: CategoryComboBox::new(),
                drive_combo_box,
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
        let category_combo_box = self.category_combo_box.view(translations);
        let action_section = self.action_section(translations);

        container(
            column![
                row![
                    drive_combo_box.map(DeleteMessage::DriveComboBox),
                    category_combo_box.map(DeleteMessage::CategoryComboBox)
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
            DeleteMessage::CategoryComboBox(msg) => match msg {
                CategoryComboBoxMessage::CategoriesFetched(categories) => {
                    self.category_combo_box.categories_per_drive = categories;
                    Task::none()
                }
                CategoryComboBoxMessage::CategorySelected(category) => {
                    self.category_combo_box.selected_category = Some(category);
                    Task::none()
                }
            },
            DeleteMessage::DriveComboBox(msg) => match msg {
                DriveComboBoxMessage::DrivesFetched(drives) => {
                    self.drive_combo_box.drives = drives;
                    Task::none()
                }
                DriveComboBoxMessage::DriveSelected(drive) => {
                    self.drive_combo_box.selected_drive = Some(drive.clone());
                    CategoryComboBox::find_categories_for_drive(self.query_use_case.clone(), drive)
                        .map(DeleteMessage::CategoryComboBox)
                }
            },
            DeleteMessage::EndDelete => {
                self.is_deleted = true;
                self.drive_combo_box.selected_drive = None;
                self.category_combo_box.selected_category = None;
                DriveComboBox::find_drives(self.query_use_case.clone())
                    .map(DeleteMessage::DriveComboBox)
            }
            DeleteMessage::StartDelete => {
                self.is_deleted = false;
                self.delete()
            }
        }
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

    fn delete(&self) -> Task<DeleteMessage> {
        let delete_use_case = self.delete_use_case.clone();
        let drive = self.drive_combo_box.selected_drive.clone().unwrap();
        let category = self.category_combo_box.selected_category.clone();
        Task::perform(
            async move {
                delete_use_case
                    .delete(&drive, category.as_deref())
                    .unwrap_or_else(popup_error);
            },
            |()| DeleteMessage::EndDelete,
        )
    }
}
