use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;

#[derive(Clone, Debug)]
pub enum DeleteMessage {
    Delete,
    DriveComboBox(DriveComboBoxMessage),
}
