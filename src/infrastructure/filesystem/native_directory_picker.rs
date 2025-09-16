use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use rfd::AsyncFileDialog;
use std::path::PathBuf;

pub struct NativeDirectoryPicker;

#[async_trait::async_trait]
impl DirectoryPicker for NativeDirectoryPicker {
    async fn pick_directory(&self) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            // On Linux, we need a runtime
            crate::config::constants::TOKIO_RUNTIME.block_on(Self::directory_picker())
        }

        #[cfg(not(target_os = "linux"))]
        {
            // On Windows/macOS, can await directly
            Self::directory_picker().await
        }
    }
}

impl NativeDirectoryPicker {
    async fn directory_picker() -> Option<PathBuf> {
        AsyncFileDialog::new()
            .set_title("Select Directory to Index")
            .pick_folder()
            .await
            .map(|handle| handle.path().to_path_buf())
    }
}
