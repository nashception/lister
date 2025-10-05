use std::io::Error;
use std::path::StripPrefixError;

#[derive(Debug, thiserror::Error)]
pub enum DirectoryScannerError {
    #[error("Relative path error: {0}")]
    RelativePath(#[from] StripPrefixError),
    #[error("File size error: {0}")]
    FileSize(#[from] Error),
}
