use crate::domain::model::file_entry::FileEntry;
use jwalk::{DirEntry, WalkDir};
use std::path::{Path, StripPrefixError};

#[derive(Debug, thiserror::Error)]
pub enum DirectoryScannerError {
    #[error("Relative path error: {0}")]
    RelativePath(#[from] StripPrefixError),
    #[error("File metadata error: {0}")]
    FileMetadata(#[from] jwalk::Error),
}

/// Recursively scans a directory and returns a list of [`FileEntry`] values.
///
/// Uses [`jwalk`](https://docs.rs/jwalk) to traverse all subdirectories,
/// filtering out directories and keeping only files.
///
/// # Errors
///
/// Returns a [`DirectoryScannerError`] if:
/// - A [`RelativePath`](DirectoryScannerError::RelativePath) error occurs when
///   stripping the base directory prefix from a file path.
/// - A [`FileMetadata`](DirectoryScannerError::FileMetadata) error occurs when retrieving
///   file metadata (e.g., file size).
pub fn scan_directory(directory: &Path) -> Result<Vec<FileEntry>, DirectoryScannerError> {
    WalkDir::new(directory)
        .skip_hidden(false)
        .sort(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| extract_file_info(&directory, &e))
        .collect()
}

fn extract_file_info(
    base_directory: &Path,
    entry: &DirEntry<((), ())>,
) -> Result<FileEntry, DirectoryScannerError> {
    Ok(FileEntry {
        path: relative_path(base_directory, &entry.path())?,
        size_bytes: entry.metadata()?.len(),
    })
}

fn relative_path(base_directory: &Path, file_path: &Path) -> Result<String, DirectoryScannerError> {
    Ok(file_path
        .strip_prefix(base_directory)
        .map(|p| p.to_string_lossy().into_owned())?)
}
