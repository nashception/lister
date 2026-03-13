use crate::domain::model::directory::DirectoryData;
use crate::domain::model::file_entry::FileEntry;

#[derive(Clone, Debug)]
pub enum WriteMessage {
    CategoryChanged(String),
    DatabaseCleaned,
    DirectoryPressed { dialog_title: String },
    DirectoryChanged(Option<DirectoryData>),
    DiskChanged(String),
    InsertInDatabaseFinished(usize),
    ResetForm,
    ScanDirectoryFinished(Vec<FileEntry>),
    WriteSubmit,
}
