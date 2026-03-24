use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;

#[derive(Clone, Debug)]
pub enum DeleteMessage {
    CategoriesFetched(Vec<String>),
    CategorySelected(String),
    DriveComboBox(DriveComboBoxMessage),
    EndDelete,
    StartDelete,
}
