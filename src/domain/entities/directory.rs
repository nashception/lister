use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct DirectoryData {
    pub drive_name: String,
    pub drive_available_space: u64,
    pub directory: PathBuf,
}

impl DirectoryData {

    pub fn last_folder_name(&self) -> String {
        self.directory
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}