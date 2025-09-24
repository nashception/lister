use std::io::Error;
use std::path::StripPrefixError;
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum DirectoryScannerError {
    #[error("Tokio error: {0}")]
    Tokio(#[from] JoinError),
    #[error("Relative path error: {0}")]
    RelativePath(#[from] StripPrefixError),
    #[error("File size error: {0}")]
    FileSize(#[from] Error),
}
