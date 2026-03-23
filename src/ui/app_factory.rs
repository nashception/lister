use crate::infrastructure::database::pool::SqliteRepositoryPool;
use crate::infrastructure::database::repository::ListerRepository;
use crate::utils::dialogs::popup_error_and_exit;
use std::sync::Arc;

#[must_use]
pub fn create() -> Arc<ListerRepository> {
    Arc::new(ListerRepository::new(
        SqliteRepositoryPool::new("app.db").unwrap_or_else(|error| popup_error_and_exit(error)),
    ))
}
