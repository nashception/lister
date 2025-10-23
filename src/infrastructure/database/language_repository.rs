use crate::domain::entities::language::Language;
use crate::infrastructure::database::pool::{RepositoryError, SqliteRepositoryPool};
use crate::infrastructure::database::schema::settings;
use diesel::prelude::*;
use diesel::{OptionalExtension, RunQueryDsl};
use std::sync::Arc;

/// Repository for managing application language settings.
pub struct LanguageRepository {
    pool: Arc<SqliteRepositoryPool>,
}

impl LanguageRepository {
    #[must_use]
    /// Creates a new [`LanguageRepository`] with the given pool.
    pub const fn new(pool: Arc<SqliteRepositoryPool>) -> Self {
        Self { pool }
    }

    /// Retrieves the current application language from the database.
    ///
    /// Returns the stored language if present; otherwise defaults to [`Language::English`].
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during query execution.
    pub fn get_language(&self) -> Result<Language, RepositoryError> {
        self.pool.execute_db_operation(|conn| {
            let lang: Option<String> = settings::table
                .filter(settings::key.eq("language"))
                .select(settings::value)
                .first(conn)
                .optional()?;

            Ok(lang.map_or_else(|| Language::English, |l| Language::new(&l)))
        })
    }

    /// Sets the application language in the database.
    ///
    /// Replaces any existing language setting with the provided value.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] if:
    /// - A [`ConnectionPool`](RepositoryError::ConnectionPool) error occurs while acquiring a connection.
    /// - A [`Database`](RepositoryError::Database) error occurs during the update operation.
    pub fn set_language(&self, language: &Language) -> Result<(), RepositoryError> {
        self.pool.execute_db_operation(|conn| {
            diesel::replace_into(settings::table)
                .values((
                    settings::key.eq("language"),
                    settings::value.eq(language.code()),
                ))
                .execute(conn)?;
            Ok(())
        })
    }
}