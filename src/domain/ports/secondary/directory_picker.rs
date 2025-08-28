use std::path::PathBuf;

#[async_trait::async_trait]
pub trait DirectoryPicker: Send + Sync {
    async fn pick_directory(&self) -> Option<PathBuf>;
}
