use crate::domain::entities::file_entry::FileEntry;
use crate::domain::errors::domain_error::DomainError;
use jwalk::{DirEntry, WalkDir};
use std::path::{Path, StripPrefixError};

#[derive(Debug, thiserror::Error)]
pub enum DirectoryScannerError {
    #[error("Relative path error: {0}")]
    RelativePath(#[from] StripPrefixError),
    #[error("File metadata error: {0}")]
    FileMetadata(#[from] jwalk::Error),
}

impl From<DirectoryScannerError> for DomainError {
    fn from(e: DirectoryScannerError) -> Self {
        Self::DirectoryScannerError(e.to_string())
    }
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
    let directory = directory.to_path_buf();

    WalkDir::new(&directory)
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
    let metadata = entry.metadata()?;
    Ok(FileEntry {
        path: relative_path(base_directory, &entry.path())?,
        size_bytes: metadata.len(),
    })
}

fn relative_path(base_directory: &Path, file_path: &Path) -> Result<String, DirectoryScannerError> {
    let relative_path = file_path
        .strip_prefix(base_directory)
        .map(|p| p.to_string_lossy().into_owned())?;
    Ok(relative_path)
}
