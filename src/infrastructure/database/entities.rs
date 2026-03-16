use crate::infrastructure::database::binary_format::UuidSqlite;
use crate::infrastructure::database::schema::{drive_entries, file_categories, file_entries};
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};

#[derive(Queryable)]
pub struct FileWithMetadataDto {
    pub category_name: String,
    pub drive_name: String,
    pub drive_available_space: i64,
    pub drive_insertion_time: NaiveDateTime,
    pub path: String,
    pub weight: i64,
}

#[derive(Insertable)]
#[diesel(table_name = file_categories)]
pub struct NewFileCategoryDto {
    pub id: UuidSqlite,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = drive_entries)]
pub struct NewDriveEntryDto {
    pub id: UuidSqlite,
    pub category_id: UuidSqlite,
    pub name: String,
    pub available_space: i64,
    pub insertion_time: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = file_entries)]
pub struct NewFileEntryDto {
    pub id: UuidSqlite,
    pub drive_id: UuidSqlite,
    pub path: String,
    pub weight: i64,
}
