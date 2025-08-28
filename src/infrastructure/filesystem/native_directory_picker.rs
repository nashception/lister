use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use rfd::AsyncFileDialog;
use std::path::PathBuf;

pub struct NativeDirectoryPicker;

#[async_trait::async_trait]
impl DirectoryPicker for NativeDirectoryPicker {
    async fn pick_directory(&self) -> Option<PathBuf> {
        AsyncFileDialog::new()
            .set_title("Select Directory to Index")
            .pick_folder()
            .await
            .map(|handle| handle.path().to_path_buf())
    }
}
