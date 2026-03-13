use crate::application::file_query_service::FileQueryService;
use crate::tr;
use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;
use crate::utils::dialogs::popup_error;
use iced::widget::pick_list;
use iced::{Element, Task};
use std::collections::HashMap;
use std::sync::Arc;

pub struct DriveComboBox {
    pub drives: Vec<String>,
    pub selected_drive: Option<String>,
}

impl DriveComboBox {
    pub fn new(query_use_case: Arc<FileQueryService>) -> (Self, Task<DriveComboBoxMessage>) {
        (
            Self {
                drives: vec![],
                selected_drive: None,
            },
            Task::perform(
                async move {
                    query_use_case.list_drive_names().unwrap_or_else(|err| {
                        popup_error(err);
                        vec![]
                    })
                },
                DriveComboBoxMessage::DrivesFetched,
            ),
        )
    }

    pub fn view(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, DriveComboBoxMessage> {
        pick_list(
            self.drives.clone(),
            self.selected_drive.clone(),
            DriveComboBoxMessage::DriveSelected,
        )
        .placeholder(tr!(translations, "select_drive_placeholder"))
        .into()
    }
}
