use crate::infrastructure::database::repository::ListerRepository;
use crate::tr;
use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;
use crate::utils::dialogs::popup_error;
use iced::widget::pick_list;
use iced::{Element, Task};
use std::collections::HashMap;
use std::sync::Arc;

pub struct DriveComboBox {
    repository: Arc<ListerRepository>,
    pub drives: Vec<String>,
    pub selected_drive: Option<String>,
}

impl DriveComboBox {
    pub fn new(repository: Arc<ListerRepository>) -> (Self, Task<DriveComboBoxMessage>) {
        let drive_combo_box = Self {
            repository,
            drives: vec![],
            selected_drive: None,
        };
        let task = drive_combo_box.find_drives();
        (drive_combo_box, task)
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

    pub fn find_drives(&self) -> Task<DriveComboBoxMessage> {
        let repository = self.repository.clone();
        Task::perform(
            async move {
                repository.find_all_drive_names().unwrap_or_else(|err| {
                    popup_error(err);
                    vec![]
                })
            },
            DriveComboBoxMessage::DrivesFetched,
        )
    }
}
