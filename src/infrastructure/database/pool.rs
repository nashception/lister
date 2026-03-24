use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

type DieselPool = Pool<ConnectionManager<SqliteConnection>>;
pub type DieselConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

#[derive(Debug, thiserror::Error)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("Connection pool error: {0}")]
    ConnectionPool(#[from] PoolError),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("Error reading file metadata: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Error deserializing json: {0}")]
    DeserializeError(#[from] serde_json::Error),
}

/// Core database pool and infrastructure for `SQLite` repositories.
///
/// Handles connection pooling, foreign key constraints, migrations,
/// and PRAGMA tuning for performance.
pub struct SqliteRepositoryPool {
    pool: DieselPool,
}

impl SqliteRepositoryPool {
    /// Creates a new [`SqliteRepositoryPool`] instance and initializes the database connection.
    ///
    /// This function sets up a connection pool to the database, enables foreign key
    /// constraints, applies necessary `SQLite` PRAGMA settings, and runs any pending
    /// database migrations before returning a fully initialized repository instance.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while creating or acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during database initialization.
    /// - A [`Migration`](InfrastructureError::Migration) error occurs while applying migrations.
    ///
    /// # Parameters
    ///
    /// - `database_url`: The database connection URL (e.g., path to the `SQLite` database file).
    ///
    /// # Returns
    ///
    /// Returns the initialized [`SqliteRepositoryPool`] instance upon success.
    pub fn new(database_url: &str) -> Result<Self, InfrastructureError> {
        let pool = Self::create_pool(database_url)?;
        let mut conn = pool.get().map_err(InfrastructureError::ConnectionPool)?;
        Self::enable_foreign_keys(&mut conn)?;
        Self::apply_pragmas(&mut conn)?;
        Self::run_migrations(&mut conn)?;
        Ok(Self { pool })
    }

    fn create_pool(database_url: &str) -> Result<DieselPool, InfrastructureError> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        Pool::builder()
            .build(manager)
            .map_err(InfrastructureError::ConnectionPool)
    }

    fn enable_foreign_keys(conn: &mut SqliteConnection) -> Result<(), InfrastructureError> {
        diesel::sql_query("PRAGMA foreign_keys = ON;")
            .execute(conn)
            .map_err(InfrastructureError::Database)?;
        Ok(())
    }

    fn apply_pragmas(conn: &mut SqliteConnection) -> Result<(), InfrastructureError> {
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
                .map_err(InfrastructureError::Database)?;
        }
        Ok(())
    }

    fn run_migrations(conn: &mut SqliteConnection) -> Result<(), InfrastructureError> {
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|err| InfrastructureError::Migration(err.to_string()))?;
        Ok(())
    }

    /// Retrieves a single database connection from the internal connection pool.
    ///
    /// This function is typically used internally by repository methods or
    /// transactional helpers to acquire a managed database connection.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    ///
    /// # Returns
    ///
    /// Returns a pooled [`DieselConnection`] on success.
    pub fn get_connection(&self) -> Result<DieselConnection, InfrastructureError> {
        self.pool.get().map_err(InfrastructureError::ConnectionPool)
    }

    /// Executes a database operation with automatic connection management.
    ///
    /// This function acquires a connection from the pool, executes the provided
    /// closure with it, and automatically handles connection release afterward.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during the operation.
    ///
    /// # Parameters
    ///
    /// - `operation`: A closure that performs the desired database action using a mutable reference to the connection.
    ///
    /// # Returns
    ///
    /// Returns the result of the provided operation if successful.
    pub fn execute_db_operation<F, R>(&self, operation: F) -> Result<R, InfrastructureError>
    where
        F: FnOnce(&mut DieselConnection) -> Result<R, InfrastructureError>,
    {
        let mut conn = self.get_connection()?;
        operation(&mut conn)
    }

    /// Executes a database operation within an **immediate transaction**.
    ///
    /// The provided closure runs within a single transactional context.
    /// If the operation returns an error, the transaction is automatically rolled back.
    ///
    /// This is particularly useful for ensuring atomic updates across multiple
    /// database statements or operations that must succeed or fail as a unit.
    ///
    /// # Errors
    ///
    /// Returns a [`InfrastructureError`] if:
    /// - A [`ConnectionPool`](InfrastructureError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](InfrastructureError::Database) error occurs during the transaction.
    ///
    /// # Parameters
    ///
    /// - `operation`: A closure that performs the transactional work on the provided connection.
    ///
    /// # Returns
    ///
    /// Returns the result of the transaction closure upon success.
    pub fn execute_in_transaction<F, R>(&self, operation: F) -> Result<R, InfrastructureError>
    where
        F: FnOnce(&mut SqliteConnection) -> Result<R, InfrastructureError>,
    {
        let mut conn = self.get_connection()?;
        conn.immediate_transaction(|conn| operation(conn))
    }
}
