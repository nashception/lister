use crate::domain::entities::file_entry::FileEntry;
use crate::domain::entities::types::Bytes;
use crate::domain::errors::scanner_error::DirectoryScannerError;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn scan_directory(directory: &Path) -> Result<Vec<FileEntry>, DirectoryScannerError> {
    let directory = directory.to_path_buf();

    WalkDir::new(&directory)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| extract_file_info(&directory, e.path()))
        .collect()
}

fn extract_file_info(
    base_directory: &Path,
    file_path: &Path,
) -> Result<FileEntry, DirectoryScannerError> {
    Ok(FileEntry {
        path: relative_path(base_directory, file_path)?,
        size_bytes: file_size(file_path)?,
    })
}

fn relative_path(base_directory: &Path, file_path: &Path) -> Result<String, DirectoryScannerError> {
    let relative_path = file_path
        .strip_prefix(base_directory)
        .map(|p| p.to_string_lossy().into_owned())?;
    Ok(relative_path)
}

fn file_size(path: &Path) -> Result<Bytes, DirectoryScannerError> {
    let file_size = fs::metadata(path).map(|m| Bytes::from(m.len()))?;
    Ok(file_size)
}
