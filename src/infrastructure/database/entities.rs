use chrono::NaiveDateTime;
use crate::infrastructure::database::schema::{drive_entries, file_categories, file_entries};
use diesel::{Associations, Identifiable, Insertable, Queryable};

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(table_name = file_categories)]
struct FileCategoryEntity {
    id: i32,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(belongs_to(FileCategoryEntity, foreign_key = category_id))]
#[diesel(table_name = drive_entries)]
struct DriveEntryEntity {
    id: i32,
    category_id: i32,
    name: String,
    remaining_space: i64,
    insertion_time: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Queryable, Identifiable, Associations)]
#[diesel(belongs_to(DriveEntryEntity, foreign_key = drive_id))]
#[diesel(table_name = file_entries)]
struct FileEntryEntity {
    id: i32,
    drive_id: i32,
    path: String,
    weight: i64,
}

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
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = drive_entries)]
pub struct NewDriveEntryDto {
    pub category_id: i32,
    pub name: String,
    pub available_space: i64,
    pub insertion_time: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = file_entries)]
pub struct NewFileEntryDto {
    pub drive_id: i32,
    pub path: String,
    pub weight: i64,
}
