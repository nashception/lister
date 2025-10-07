use crate::domain::entities::directory::DirectoryData;

pub trait DirectoryPicker: Send + Sync {
    fn pick_directory(&self, title: &str) -> Option<DirectoryData>;
}
