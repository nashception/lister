use crate::domain::entities::file_entry::FileWithMetadata;

#[derive(Clone, Debug)]
pub struct PaginatedResult {
    pub items: Vec<FileWithMetadata>,
    pub total_count: u64,
}
