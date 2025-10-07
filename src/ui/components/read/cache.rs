use crate::domain::entities::file_entry::FileWithMetadata;

pub struct Cache {
    pub drive: Option<String>,
    pub query: Option<String>,
    pub results: Option<Vec<FileWithMetadata>>,
}

impl Cache {
    pub const fn new() -> Self {
        Self {
            drive: None,
            query: None,
            results: None,
        }
    }

    pub fn clear(&mut self) {
        self.drive = None;
        self.query = None;
        self.results = None;
    }

    pub fn store(&mut self, drive: Option<String>, query: String, results: Vec<FileWithMetadata>) {
        self.drive = drive;
        self.query = Some(query);
        self.results = Some(results);
    }

    pub fn is_valid_for(&self, selected_drive: Option<&String>, query: &str) -> bool {
        self.drive.as_ref() == selected_drive && self.query.as_deref() == Some(query)
    }

    pub fn get_page(
        &self,
        selected_drive: Option<&String>,
        query: &str,
        page_index: usize,
        items_per_page: usize,
    ) -> Option<Vec<FileWithMetadata>> {
        if self.is_valid_for(selected_drive, query)
            && let Some(results) = &self.results
        {
            let start = page_index * items_per_page;
            if start < results.len() {
                let end = (start + items_per_page).min(results.len());
                return Some(results[start..end].to_vec());
            }
        }
        None
    }
}
