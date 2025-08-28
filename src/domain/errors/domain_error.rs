use crate::domain::errors::repository_error::RepositoryError;
use crate::domain::errors::scanner_error::DirectoryScannerError;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Directory scan failed: {0}")]
    DirectoryScannerError(#[from] DirectoryScannerError),
}