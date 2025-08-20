use crate::schema::file_entries::CategoryId;
use crate::schema::{file_categories, file_entries};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
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

    let mut lister_repository = DieselListerRepository::new(pool);

    let new_category_id = lister_repository.add_category(new_cat);
    let another_new_category_id = lister_repository.add_category(another_new_cat);

    let rows = lister_repository.find_all_categories();

    let expected = vec![
        FileCategoryEntity {
            id: new_category_id as i32,
            name: "Cat Videos".into(),
        },
        FileCategoryEntity {
            id: another_new_category_id as i32,
            name: "Games".into(),
        },
    ];

    assert_eq!(rows, expected);

    lister_repository.add_files(vec![
        NewFileEntry {
            category_id: 1,
            path: "Dr House/Season 1/Episode 1.mkv",
            weight: 2000000,
        },
        NewFileEntry {
            category_id: 1,
            path: "Dr House/Season 1/Episode 2.mkv",
            weight: 2500000,
        },
        NewFileEntry {
            category_id: 1,
            path: "Dr House/Season 1/Episode 3.mkv",
            weight: 3000000,
        },
        NewFileEntry {
            category_id: 2,
            path: "Red Dead Redemption Remastered",
            weight: 1500000,
        },
    ]);

    let files = lister_repository.find_all_files();

    assert_eq!(
        files,
        vec![
            FileEntryEntity {
                id: 1,
                category_id: 1,
                path: String::from("Dr House/Season 1/Episode 1.mkv"),
                weight: 2000000,
            },
            FileEntryEntity {
                id: 2,
                category_id: 1,
                path: String::from("Dr House/Season 1/Episode 2.mkv"),
                weight: 2500000,
            },
            FileEntryEntity {
                id: 3,
                category_id: 1,
                path: String::from("Dr House/Season 1/Episode 3.mkv"),
                weight: 3000000,
            },
            FileEntryEntity {
                id: 4,
                category_id: 2,
                path: String::from("Red Dead Redemption Remastered"),
                weight: 1500000,
            },
        ]
    );

    let files_by_category = lister_repository.find_all_files_by_category_id(2);
    assert_eq!(
        files_by_category,
        vec![FileEntryEntity {
            id: 4,
            category_id: 2,
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
struct FileEntry {
    category_id: u64,
    path: String,
    weight: u64,
}

trait ListerRepository {
    fn add_category(&mut self, category: NewFileCategory) -> u32;
    fn add_files(&mut self, files: Vec<NewFileEntry>);
    fn find_all_categories(&self) -> Vec<FileCategoryEntity>;
    fn find_all_files_by_category_id(&self, category_id: u32) -> Vec<FileEntryEntity>;
    fn find_all_files(&self) -> Vec<FileEntryEntity>;
}

struct DieselListerRepository {
    pool: DieselPool,
}

impl DieselListerRepository {
    fn new(pool: DieselPool) -> Self {
        DieselListerRepository { pool }
    }
}

impl ListerRepository for DieselListerRepository {
    fn add_category(&mut self, category: NewFileCategory) -> u32 {
        let conn = &mut self.pool.get().expect("DB pool get failed");
        diesel::insert_into(file_categories::table)
            .values(category)
            .returning(file_categories::Id)
            .get_result::<i32>(conn)
            .map(|id| id as u32)
            .expect("insert category failed")
    }

    fn add_files(&mut self, files: Vec<NewFileEntry>) {
        let conn = &mut self.pool.get().expect("DB pool get failed");
        diesel::insert_into(file_entries::table)
            .values(&files)
            .execute(conn)
            .expect("insert file batch failed");
    }

    fn find_all_categories(&self) -> Vec<FileCategoryEntity> {
        let conn = &mut self.pool.get().expect("DB pool get failed");
        file_categories::table.load(conn).unwrap()
    }

    fn find_all_files_by_category_id(&self, category_id: u32) -> Vec<FileEntryEntity> {
        let conn = &mut self.pool.get().expect("DB pool get failed");
        let category_id_i32 = category_id as i32;
        let entities = file_entries::table
            .filter(CategoryId.eq(category_id_i32))
            .load::<FileEntryEntity>(conn)
            .expect("load files failed");
        entities.into_iter().map(|e| e.into()).collect()
    }

    fn find_all_files(&self) -> Vec<FileEntryEntity> {
        let conn = &mut self.pool.get().expect("DB pool get failed");
        file_entries::table.load(conn).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(table_name = file_categories)]
pub struct FileCategoryEntity {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(FileCategoryEntity, foreign_key = CategoryId))]
#[diesel(table_name = file_entries)]
pub struct FileEntryEntity {
    pub id: i32,
    #[diesel(column_name = "CategoryId")]
    pub category_id: i32,
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
#[diesel(table_name = file_entries)]
pub struct NewFileEntry<'a> {
    #[diesel(column_name = "CategoryId")]
    pub category_id: i32,
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
            category_id: row.category_id as u64,
            path: row.path,
            weight: row.weight as u64,
        }
    }
}
