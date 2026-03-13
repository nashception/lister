#[derive(Debug, Clone)]
pub enum DriveComboBoxMessage {
    DrivesFetched(Vec<String>),
    DriveSelected(String),
}