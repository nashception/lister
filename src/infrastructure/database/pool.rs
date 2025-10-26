use crate::config::constants::MIGRATIONS;
use crate::domain::errors::domain_error::DomainError;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};
use diesel_migrations::MigrationHarness;
use std::sync::Arc;

type DieselPool = Pool<ConnectionManager<SqliteConnection>>;
pub type DieselConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("Connection pool error: {0}")]
    ConnectionPool(#[from] PoolError),
    #[error("Migration error: {0}")]
    Migration(String),
}

impl From<RepositoryError> for DomainError {
    fn from(e: RepositoryError) -> Self {
        Self::RepositoryFailure(e.to_string())
    }
}

/// Core database pool and infrastructure for `SQLite` repositories.
///
/// Handles connection pooling, foreign key constraints, migrations,
/// and PRAGMA tuning for performance.
pub struct SqliteRepositoryPool {
    pool: DieselPool,
}

impl SqliteRepositoryPool {
    pub fn new(database_url: &str) -> Result<Arc<Self>, RepositoryError> {
        let pool = Self::create_pool(database_url)?;
        {
            let mut conn = pool.get().map_err(RepositoryError::ConnectionPool)?;
            Self::enable_foreign_keys(&mut conn)?;
            Self::apply_pragmas(&mut conn)?;
            Self::run_migrations(&mut conn)?;
        }
        Ok(Arc::new(Self { pool }))
    }

    fn create_pool(database_url: &str) -> Result<DieselPool, RepositoryError> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        Pool::builder()
            .build(manager)
            .map_err(RepositoryError::ConnectionPool)
    }

    fn enable_foreign_keys(conn: &mut SqliteConnection) -> Result<(), RepositoryError> {
        diesel::sql_query("PRAGMA foreign_keys = ON;")
            .execute(conn)
            .map_err(RepositoryError::Database)?;
        Ok(())
    }

    fn apply_pragmas(conn: &mut SqliteConnection) -> Result<(), RepositoryError> {
        let pragmas = [
            "PRAGMA journal_mode = WAL;",
            "PRAGMA synchronous = NORMAL;",
            "PRAGMA cache_size = -80000;", // ~80MB cache
            "PRAGMA temp_store = MEMORY;",
            "PRAGMA locking_mode = EXCLUSIVE;",
        ];
        for pragma in pragmas {
            diesel::sql_query(pragma)
                .execute(conn)
                .map_err(RepositoryError::Database)?;
        }
        Ok(())
    }

    fn run_migrations(conn: &mut SqliteConnection) -> Result<(), RepositoryError> {
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|err| RepositoryError::Migration(err.to_string()))?;
        Ok(())
    }

    /// Gets a connection from the pool.
    pub(crate) fn get_connection(&self) -> Result<DieselConnection, RepositoryError> {
        self.pool.get().map_err(RepositoryError::ConnectionPool)
    }

    /// Executes a database operation with automatic connection management.
    pub(crate) fn execute_db_operation<F, R>(&self, operation: F) -> Result<R, RepositoryError>
    where
        F: FnOnce(&mut DieselConnection) -> Result<R, RepositoryError>,
    {
        let mut conn = self.get_connection()?;
        operation(&mut conn)
    }

    /// Executes a database operation within an immediate transaction.
    ///
    /// Note: The closure receives `&mut SqliteConnection` directly because
    /// `immediate_transaction` dereferences the pooled connection.
    pub(crate) fn execute_in_transaction<F, R>(&self, operation: F) -> Result<R, RepositoryError>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<R, RepositoryError>,
    {
        let mut conn = self.get_connection()?;
        conn.immediate_transaction(|conn| operation(conn))
    }
}
