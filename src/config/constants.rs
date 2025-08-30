use crate::utils::dialogs::popup_error_and_exit;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use std::sync::LazyLock;
use tokio::runtime::Runtime;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
pub const ITEMS_PER_PAGE: usize = 100;
pub const CACHED_SIZE: i64 = 10000;

pub static TOKIO_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .build()
        .unwrap_or_else(|err| popup_error_and_exit(err))
});
