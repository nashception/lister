use diesel_migrations::{embed_migrations, EmbeddedMigrations};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
pub const ITEMS_PER_PAGE: usize = 100;
pub const CACHED_SIZE: i64 = 10000;
