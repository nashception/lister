#[derive(Clone, Debug, PartialEq)]
pub struct Language {
    code: String,
}

impl Language {
    pub fn new(code: &str) -> Self {
        let normalized_code = match code.to_lowercase().as_str() {
            "en" | "english" => "en",
            "fr" | "french" => "fr",
            _ => "en",
        };
        Self {
            code: normalized_code.to_string(),
        }
    }

    pub fn english() -> Self {
        Self::new("en")
    }

    pub fn french() -> Self {
        Self::new("fr")
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn toggle(&self) -> Self {
        match self.code.as_str() {
            "en" => Self::french(),
            "fr" => Self::english(),
            _ => Self::english(),
        }
    }
}