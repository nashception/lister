use crate::application::file_query_service::FileQueryService;
use crate::tr;
use crate::ui::messages::category_combo_box::CategoryComboBoxMessage;
use crate::utils::dialogs::popup_error;
use iced::widget::pick_list;
use iced::{Element, Task};
use std::collections::HashMap;
use std::sync::Arc;

pub struct CategoryComboBox {
    pub categories_per_drive: Vec<String>,
    pub selected_category: Option<String>,
}

impl CategoryComboBox {
    pub const fn new() -> Self {
        Self {
            categories_per_drive: vec![],
            selected_category: None,
        }
    }

    pub fn view(
        &'_ self,
        translations: &HashMap<String, String>,
    ) -> Element<'_, CategoryComboBoxMessage> {
        pick_list(
            self.categories_per_drive.clone(),
            self.selected_category.clone(),
            CategoryComboBoxMessage::CategorySelected,
        )
        .placeholder(tr!(translations, "select_category_placeholder"))
        .into()
    }

    pub fn find_categories_for_drive(
        query_use_case: Arc<FileQueryService>,
        drive: String,
    ) -> Task<CategoryComboBoxMessage> {
        Task::perform(
            async move {
                query_use_case
                    .list_category_names_for_drive(&drive)
                    .unwrap_or_else(|err| {
                        popup_error(err);
                        vec![]
                    })
            },
            CategoryComboBoxMessage::CategoriesFetched,
        )
    }
}
