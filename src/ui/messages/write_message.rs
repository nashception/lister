use crate::domain::entities::directory::DirectoryData;
use crate::domain::entities::file_entry::FileEntry;

#[derive(Clone, Debug)]
pub enum WriteMessage {
    DirectoryPressed,
    DirectoryChanged(Option<DirectoryData>),
    DriveAlreadyExistChecked(bool),
    CategoryChanged(String),
    DiskChanged(String),
    DatabaseCleaned,
    WriteSubmit,
    ScanDirectoryFinished(Vec<FileEntry>),
    InsertInDatabaseFinished(usize),
    ResetForm,
}
