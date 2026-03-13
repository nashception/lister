use crate::ui::messages::category_combo_box::CategoryComboBoxMessage;
use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;

#[derive(Clone, Debug)]
pub enum DeleteMessage {
    CategoryComboBox(CategoryComboBoxMessage),
    DriveComboBox(DriveComboBoxMessage),
    EndDelete,
    StartDelete,
}
