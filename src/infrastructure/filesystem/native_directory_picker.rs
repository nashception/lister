use crate::domain::entities::directory::DirectoryData;
use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use crate::infrastructure::filesystem::directory::directory_data;
use rfd::AsyncFileDialog;

pub struct NativeDirectoryPicker;

#[async_trait::async_trait]
impl DirectoryPicker for NativeDirectoryPicker {
    async fn pick_directory(&self) -> Option<DirectoryData> {
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
    async fn directory_picker() -> Option<DirectoryData> {
        AsyncFileDialog::new()
            .set_title("Select Directory to Index")
            .pick_folder()
            .await
            .map(|handle| directory_data(handle.path()))
    }
}
