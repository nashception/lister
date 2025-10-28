use crate::domain::model::file_entry::FileWithMetadata;
use crate::infrastructure::database::entities::FileWithMetadataDto;

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
