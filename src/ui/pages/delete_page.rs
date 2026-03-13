use crate::application::delete_service::DeleteService;
use crate::application::file_query_service::FileQueryService;
use crate::ui::components::read::drive_combo_box::DriveComboBox;
use crate::ui::messages::delete_message::DeleteMessage;
use crate::utils::dialogs::popup_error;
use iced::widget::column;
use iced::{Element, Task};
use std::collections::HashMap;
use std::sync::Arc;
use crate::tr;

pub struct DeletePage {
    delete_use_case: Arc<DeleteService>,
    query_use_case: Arc<FileQueryService>,
    drive_combo_box: DriveComboBox,
    category: Option<String>,
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
                drive_combo_box,
                category: None,
            },
            combo_box_task.map(DeleteMessage::DriveComboBox),
        )
    }

    pub fn title(translations: &HashMap<String, String>) -> String {
        tr!(translations, "delete_page_title")
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, DeleteMessage> {
        column![].spacing(20).padding(20).into()
    }

    fn delete(&self) -> Task<DeleteMessage> {
        let delete_use_case = self.delete_use_case.clone();
        let drive = self.drive_combo_box.selected_drive.clone().unwrap();
        let category = self.category.clone();

        Task::perform(
            async move {
                delete_use_case
                    .delete(&drive, category.as_deref())
                    .unwrap_or_else(popup_error);
            },
            |()| DeleteMessage::Delete,
        )
    }
}
