use crate::domain::model::directory::DirectoryData;
use crate::domain::model::file_entry::FileEntry;

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
