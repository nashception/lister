use crate::domain::entities::directory::DirectoryData;
use crate::domain::entities::file_entry::FileEntry;

#[derive(Clone, Debug)]
pub enum WriteMessage {
    DirectoryPressed { dialog_title: String},
    DirectoryChanged(Option<DirectoryData>),
    CategoryChanged(String),
    DiskChanged(String),
    DatabaseCleaned,
    WriteSubmit,
    ScanDirectoryFinished(Vec<FileEntry>),
    InsertInDatabaseFinished(usize),
    ResetForm,
}
