use std::io::Error;
use std::path::StripPrefixError;

#[derive(Debug, thiserror::Error)]
pub enum DirectoryScannerError {
    #[error("Relative path error: {0}")]
    RelativePath(#[from] StripPrefixError),
    #[error("File metadata error: {0}")]
    FileMetadata(#[from] Error),
}
