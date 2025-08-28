use diesel::r2d2::PoolError;
use diesel::result::Error;
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] Error),
    #[error("Connection pool error: {0}")]
    ConnectionPool(#[from] PoolError),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("Tokio error: {0}")]
    Tokio(#[from] JoinError),
}
