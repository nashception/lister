#[derive(Eq, PartialEq)]
pub enum IndexingState {
    Ready,
    CleaningDatabase,
    Scanning,
    Saving,
    Completed { files_indexed: usize },
}

impl IndexingState {
    pub const fn is_indexing(&self) -> bool {
        matches!(self, Self::CleaningDatabase | Self::Scanning | Self::Saving)
    }
}
