#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Repository error: {0}")]
    RepositoryFailure(String),
    #[error("Directory scan failed: {0}")]
    DirectoryScannerError(String),
}
