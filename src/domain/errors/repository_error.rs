use diesel::r2d2::PoolError;
use diesel::result::Error;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] Error),
    #[error("Connection pool error: {0}")]
    ConnectionPool(#[from] PoolError),
    #[error("Migration error: {0}")]
    Migration(String),
}
