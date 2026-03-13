#[derive(Debug, Clone)]
pub enum CategoryComboBoxMessage {
    CategoriesFetched(Vec<String>),
    CategorySelected(String),
}