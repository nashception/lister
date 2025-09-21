#[derive(PartialEq)]
pub enum IndexingState {
    Ready,
    CleaningDatabase,
    Scanning,
    Saving,
    Completed { files_indexed: usize },
}

impl IndexingState {
    pub fn is_indexing(&self) -> bool {
        matches!(
            self,
            IndexingState::CleaningDatabase | IndexingState::Scanning | IndexingState::Saving
        )
    }
}
