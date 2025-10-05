use crate::domain::entities::directory::DirectoryData;
use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use crate::infrastructure::filesystem::directory::directory_data;

#[cfg(target_os = "linux")]
mod linux_runtime {
    use crate::utils::dialogs::popup_error_and_exit;
    use std::sync::LazyLock;
    use tokio::runtime::{Builder, Runtime};

    pub static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
        Builder::new_multi_thread()
            .enable_io()
            .build()
            .unwrap_or_else(|err| popup_error_and_exit(err))
    });
}

pub struct NativeDirectoryPicker;

impl DirectoryPicker for NativeDirectoryPicker {
    fn pick_directory(&self) -> Option<DirectoryData> {
        #[cfg(target_os = "linux")]
        {
            linux_runtime::TOKIO_RUNTIME.block_on(async {
                rfd::AsyncFileDialog::new()
                    .pick_folder()
                    .await
                    .map(|handle| directory_data(handle.path()))
            })
        }

        #[cfg(target_os = "windows")]
        {
            rfd::FileDialog::new()
                .pick_folder()
                .map(|handle| directory_data(handle.as_path()))
        }
    }
}
