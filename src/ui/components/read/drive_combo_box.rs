use crate::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use crate::tr;
use crate::ui::messages::read_message::ReadMessage;
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
    pub fn new(query_use_case: Arc<dyn FileQueryUseCase>) -> (Self, Task<ReadMessage>) {
        (
            Self {
                drives: vec![],
                selected_drive: None,
            },
            Task::perform(
                async move {
                    query_use_case
                        .list_drive_names()
                        .unwrap_or_else(|err| {
                            popup_error(err);
                            vec![]
                        })
                },
                ReadMessage::DrivesFetched,
            ),
        )
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, ReadMessage> {
        pick_list(
            self.drives.clone(),
            self.selected_drive.clone(),
            ReadMessage::DriveSelected,
        )
        .placeholder(tr!(translations, "select_drive_placeholder"))
        .into()
    }
}
