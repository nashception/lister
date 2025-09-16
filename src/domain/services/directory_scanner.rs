use crate::config::constants::TOKIO_RUNTIME;
use crate::domain::entities::file_entry::FileEntry;
use crate::domain::errors::scanner_error::DirectoryScannerError;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub async fn scan_directory(directory: &PathBuf) -> Result<Vec<FileEntry>, DirectoryScannerError> {
    let directory = directory.clone();

    TOKIO_RUNTIME
        .handle()
        .spawn_blocking(move || {
            WalkDir::new(&directory)
                .sort_by_file_name()
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| extract_file_info(&directory, e.path()))
                .collect()
        })
        .await?
}

fn extract_file_info(
    base_directory: &PathBuf,
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

fn file_size(path: &Path) -> Result<i64, DirectoryScannerError> {
    let file_size = fs::metadata(path).map(|m| m.len() as i64)?;
    Ok(file_size)
}
