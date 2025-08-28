use crate::domain::entities::file_entry::FileEntry;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum WriteMessage {
    CategoryChanged(String),
    DriveChanged(String),
    DirectoryPressed,
    DirectoryChanged(Option<PathBuf>),
    DatabaseCleaned,
    WriteSubmit,
    ScanDirectoryFinished(Vec<FileEntry>),
    InsertInDatabaseFinished(usize),
    ResetForm,
}
