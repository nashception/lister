use crate::application::delete_service::DeleteService;
use crate::ui::messages::delete_message::DeleteMessage;
use crate::utils::dialogs::popup_error;
use iced::Task;
use std::sync::Arc;

pub struct DeletePage {
    delete_use_case: Arc<DeleteService>,
    drive: String,
    category: Option<String>,
}

impl DeletePage {
    pub const fn new(delete_use_case: Arc<DeleteService>) -> Self {
        Self {
            delete_use_case,
            drive: String::new(),
            category: None,
        }
    }

    fn delete(&self) -> Task<DeleteMessage> {
        let delete_use_case = self.delete_use_case.clone();
        let drive = self.drive.clone();
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
