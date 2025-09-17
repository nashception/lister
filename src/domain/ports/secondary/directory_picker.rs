use crate::domain::entities::directory::DirectoryData;

#[async_trait::async_trait]
pub trait DirectoryPicker: Send + Sync {
    async fn pick_directory(&self) -> Option<DirectoryData>;
}
