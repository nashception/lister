#[derive(Clone, Debug, PartialEq)]
pub enum Language {
    English,
    French,
}

impl Language {
    pub fn new(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "en" => Language::English,
            "fr" => Language::French,
            _ => Language::English,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Language::English => "en",
            Language::French => "fr",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Language::English => Language::French,
            Language::French => Language::English,
        }
    }
}
