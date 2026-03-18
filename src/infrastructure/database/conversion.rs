use crate::domain::model::file_entry::{FileEntry, FileWithMetadata};
use crate::infrastructure::database::binary_format::UuidSqlite;
use crate::infrastructure::database::entities::{FileWithMetadataDto, NewFileEntryDto};
use uuid::Uuid;

pub trait ToI64 {
    fn to_i64_or_zero(self) -> i64;
}

pub trait ToU64 {
    fn to_u64_or_zero(self) -> u64;
}

impl ToI64 for u64 {
    fn to_i64_or_zero(self) -> i64 {
        i64::try_from(self).unwrap_or(0)
    }
}

impl ToU64 for i64 {
    fn to_u64_or_zero(self) -> u64 {
        u64::try_from(self).unwrap_or(0)
    }
}

impl From<FileWithMetadataDto> for FileWithMetadata {
    fn from(dto: FileWithMetadataDto) -> Self {
        Self {
            category_name: dto.category_name,
            drive_name: dto.drive_name,
            drive_available_space: dto.drive_available_space.to_u64_or_zero(),
            drive_insertion_time: dto.drive_insertion_time,
            path: dto.path,
            size_bytes: dto.weight.to_u64_or_zero(),
        }
    }
}

impl From<(&FileEntry, UuidSqlite)> for NewFileEntryDto {
    fn from((file, drive_id): (&FileEntry, UuidSqlite)) -> Self {
        Self {
            id: UuidSqlite(Uuid::now_v7()),
            drive_id,
            path: file.path.clone(),
            weight: file.size_bytes.to_i64_or_zero(),
        }
    }
}
