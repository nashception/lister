use crate::domain::errors::domain_error::DomainError;
use crate::infrastructure::database::command_repository::CommandRepository;
use std::sync::Arc;

pub struct DeleteService {
    command_repo: Arc<CommandRepository>,
}

impl DeleteService {
    #[must_use]
    pub const fn new(command_repo: Arc<CommandRepository>) -> Self {
        Self { command_repo }
    }

    /// Deletes files from the specified drive, optionally filtered by category.
    ///
    /// This method delegates the deletion operation to the [`CommandRepository`].
    ///
    /// # Parameters
    ///
    /// - `drive` - The name of the drive from which files should be deleted.
    /// - `category` - Optional category filter. If provided, only files matching this category will be deleted.
    ///
    /// # Errors
    ///
    /// Returns a [`DomainError`] if:
    /// - A [`Repository`](DomainError::Repository) error occurs while performing the deletion operation.
    pub fn delete(&self, drive: &str, category: Option<&str>) -> Result<(), DomainError> {
        self.command_repo
            .delete(drive, category)
            .map_err(DomainError::from)
    }
}
