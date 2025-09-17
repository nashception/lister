use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct DirectoryData {
    pub drive_name: String,
    pub drive_available_space: u64,
    pub directory: PathBuf,
}