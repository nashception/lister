use crate::schema::file_entries::{CategoryId, DriveId};
use crate::schema::{drive_entries, file_categories, file_entries};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as DieselError;
use diesel::ExpressionMethods;
use diesel::{Associations, Identifiable, Insertable, Queryable, RunQueryDsl, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
pub type DieselPool = Pool<ConnectionManager<SqliteConnection>>;

fn main() {
    println!("Hello, world!");
}

#[test]
fn lister_repository_with_database() {
    use diesel::sqlite::SqliteConnection;

    fn get_connection_pool(database_url: &str) -> DieselPool {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create SQLite pool");

        {
            let conn = &mut pool.get().expect("Failed to get connection from pool");
            diesel::sql_query("PRAGMA foreign_keys = ON")
                .execute(conn)
                .expect("Failed to enable foreign keys");
        }

        run_migrations(&pool);

        pool
    }

    fn run_migrations(pool: &Pool<ConnectionManager<SqliteConnection>>) {
        let mut conn = pool.get().expect("Failed to get connection from pool");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Migration failed");
    }

    let pool = get_connection_pool("file:memdb1?mode=memory&cache=shared");

    let new_cat = NewFileCategory { name: "Cat Videos" };
    let another_new_cat = NewFileCategory { name: "Games" };
    let new_drive = NewDriveEntry {
        name: "Windows Drive",
    };
    let new_files = vec![
        NewFileEntry {
            category_id: 1,
            drive_id: 1,
            path: "Dr House/Season 1/Episode 1.mkv",
            weight: 2000000,
        },
        NewFileEntry {
            category_id: 1,
            drive_id: 1,
            path: "Dr House/Season 1/Episode 2.mkv",
            weight: 2500000,
        },
        NewFileEntry {
            category_id: 1,
            drive_id: 1,
            path: "Dr House/Season 1/Episode 3.mkv",
            weight: 3000000,
        },
        NewFileEntry {
            category_id: 2,
            drive_id: 1,
            path: "Red Dead Redemption Remastered",
            weight: 1500000,
        },
    ];

    let lister_repository = DieselListerRepository::new(pool);

    let new_category_id = lister_repository.add_category(new_cat).unwrap();
    let another_new_category_id = lister_repository.add_category(another_new_cat).unwrap();

    let rows = lister_repository.find_all_categories().unwrap();

    let expected = vec![
        FileCategoryEntity {
            id: new_category_id,
            name: "Cat Videos".into(),
        },
        FileCategoryEntity {
            id: another_new_category_id,
            name: "Games".into(),
        },
    ];

    assert_eq!(rows, expected);

    let new_drive_id = lister_repository.add_drive(new_drive).unwrap();

    let rows = lister_repository.find_all_drives().unwrap();

    let expected = vec![DriveEntryEntity {
        id: new_drive_id,
        name: "Windows Drive".into(),
    }];

    assert_eq!(rows, expected);

    lister_repository.add_files(new_files).unwrap();

    let files = lister_repository.find_all_files().unwrap();

    assert_eq!(
        files,
        vec![
            FileEntryEntity {
                id: 1,
                category_id: 1,
                drive_id: 1,
                path: String::from("Dr House/Season 1/Episode 1.mkv"),
                weight: 2000000,
            },
            FileEntryEntity {
                id: 2,
                category_id: 1,
                drive_id: 1,
                path: String::from("Dr House/Season 1/Episode 2.mkv"),
                weight: 2500000,
            },
            FileEntryEntity {
                id: 3,
                category_id: 1,
                drive_id: 1,
                path: String::from("Dr House/Season 1/Episode 3.mkv"),
                weight: 3000000,
            },
            FileEntryEntity {
                id: 4,
                category_id: 2,
                drive_id: 1,
                path: String::from("Red Dead Redemption Remastered"),
                weight: 1500000,
            },
        ]
    );

    let files_by_category = lister_repository.find_all_files_by_category_id(2).unwrap();
    assert_eq!(
        files_by_category,
        vec![FileEntryEntity {
            id: 4,
            category_id: 2,
            drive_id: 1,
            path: String::from("Red Dead Redemption Remastered"),
            weight: 1500000,
        },]
    )
}

#[derive(Clone, Debug, PartialEq)]
struct FileCategory {
    name: String,
}

#[derive(Clone, Debug, PartialEq)]
struct DriveEntry {
    name: String,
}

#[derive(Clone, Debug, PartialEq)]
struct FileEntry {
    category_id: i32,
    drive_id: i32,
    path: String,
    weight: i64,
}

trait ListerRepository {
    fn add_category(&self, category: NewFileCategory<'_>) -> RepoResult<i32>;
    fn add_drive(&self, drive: NewDriveEntry<'_>) -> RepoResult<i32>;
    fn add_files(&self, files: Vec<NewFileEntry<'_>>) -> RepoResult<()>;
    fn find_all_categories(&self) -> RepoResult<Vec<FileCategoryEntity>>;
    fn find_all_drives(&self) -> RepoResult<Vec<DriveEntryEntity>>;
    fn find_all_files_by_category_id(&self, category_id: i32) -> RepoResult<Vec<FileEntryEntity>>;
    fn find_all_files_by_category_id_and_drive_id(
        &self,
        drive_id: i32,
        category_id: i32,
    ) -> RepoResult<Vec<FileEntryEntity>>;
    fn find_all_files(&self) -> RepoResult<Vec<FileEntryEntity>>;
}

struct DieselListerRepository {
    pool: DieselPool,
}

impl DieselListerRepository {
    fn new(pool: DieselPool) -> Self {
        DieselListerRepository { pool }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("DB error: {0}")]
    Diesel(#[from] DieselError),

    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
}

pub type RepoResult<T> = Result<T, RepoError>;

impl ListerRepository for DieselListerRepository {
    fn add_category(&self, category: NewFileCategory<'_>) -> RepoResult<i32> {
        let mut conn = self.pool.get()?;
        let id = diesel::insert_into(file_categories::table)
            .values(category)
            .returning(file_categories::Id)
            .get_result(&mut conn)?;
        Ok(id)
    }

    fn add_drive(&self, drive: NewDriveEntry<'_>) -> RepoResult<i32> {
        let mut conn = self.pool.get()?;
        let id = diesel::insert_into(drive_entries::table)
            .values(drive)
            .returning(drive_entries::Id)
            .get_result(&mut conn)?;
        Ok(id)
    }

    fn add_files(&self, files: Vec<NewFileEntry<'_>>) -> RepoResult<()> {
        let mut conn = self.pool.get()?;
        conn.immediate_transaction::<_, RepoError, _>(|conn| {
            diesel::insert_into(file_entries::table)
                .values(&files)
                .execute(conn)?;
            Ok(())
        })?;
        Ok(())
    }

    fn find_all_categories(&self) -> RepoResult<Vec<FileCategoryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = file_categories::table.load(&mut conn)?;
        Ok(rows)
    }

    fn find_all_drives(&self) -> RepoResult<Vec<DriveEntryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = drive_entries::table.load(&mut conn)?;
        Ok(rows)
    }

    fn find_all_files_by_category_id(&self, category_id: i32) -> RepoResult<Vec<FileEntryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = file_entries::table
            .filter(CategoryId.eq(category_id))
            .load::<FileEntryEntity>(&mut conn)?;
        Ok(rows)
    }

    fn find_all_files_by_category_id_and_drive_id(
        &self,
        drive_id: i32,
        category_id: i32,
    ) -> RepoResult<Vec<FileEntryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = file_entries::table
            .filter(DriveId.eq(drive_id))
            .filter(CategoryId.eq(category_id))
            .load::<FileEntryEntity>(&mut conn)?;
        Ok(rows)
    }

    fn find_all_files(&self) -> RepoResult<Vec<FileEntryEntity>> {
        let mut conn = self.pool.get()?;
        let rows = file_entries::table.load(&mut conn)?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(table_name = file_categories)]
pub struct FileCategoryEntity {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(table_name = drive_entries)]
pub struct DriveEntryEntity {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(FileCategoryEntity, foreign_key = CategoryId))]
#[diesel(belongs_to(DriveEntryEntity, foreign_key = DriveId))]
#[diesel(table_name = file_entries)]
pub struct FileEntryEntity {
    pub id: i32,
    #[diesel(column_name = "CategoryId")]
    pub category_id: i32,
    #[diesel(column_name = "DriveId")]
    pub drive_id: i32,
    pub path: String,
    pub weight: i64,
}

#[derive(Insertable)]
#[diesel(table_name = file_categories)]
pub struct NewFileCategory<'a> {
    #[diesel(column_name = "Name")]
    pub name: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = drive_entries)]
pub struct NewDriveEntry<'a> {
    #[diesel(column_name = "Name")]
    pub name: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = file_entries)]
pub struct NewFileEntry<'a> {
    #[diesel(column_name = "CategoryId")]
    pub category_id: i32,
    #[diesel(column_name = "DriveId")]
    pub drive_id: i32,
    #[diesel(column_name = "Path")]
    pub path: &'a str,
    #[diesel(column_name = "Weight")]
    pub weight: i64,
}

impl From<FileCategoryEntity> for FileCategory {
    fn from(row: FileCategoryEntity) -> Self {
        Self { name: row.name }
    }
}

impl From<FileEntryEntity> for FileEntry {
    fn from(row: FileEntryEntity) -> Self {
        Self {
            category_id: row.category_id,
            drive_id: row.drive_id,
            path: row.path,
            weight: row.weight,
        }
    }
}
