use std::path::Path;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub size_bytes: i64,
}

#[derive(Clone, Debug)]
pub struct FileWithMetadata {
    pub category_name: String,
    pub drive_name: String,
    pub path: String,
    pub size_bytes: i64,
}

impl FileWithMetadata {
    pub fn parent_directory(&self) -> String {
        Path::new(&self.path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn filename(&self) -> String {
        Path::new(&self.path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}