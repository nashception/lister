use crate::domain::model::directory::DirectoryData;
use crate::infrastructure::filesystem::directory::directory_data;

pub struct NativeDirectoryPicker;

impl NativeDirectoryPicker {
    #[must_use]
    pub fn pick_directory(&self, title: &str) -> Option<DirectoryData> {
        rfd::FileDialog::new()
            .set_title(title)
            .pick_folder()
            .map(|handle| directory_data(handle.as_path()))
    }
}
