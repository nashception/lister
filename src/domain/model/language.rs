use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Language {
    #[default]
    English,
    French,
}

impl Language {
    #[must_use]
    pub fn new(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "fr" => Self::French,
            _ => Self::English,
        }
    }

    #[must_use]
    pub const fn code(&self) -> &str {
        match self {
            Self::English => "en",
            Self::French => "fr",
        }
    }

    #[must_use]
    pub const fn toggle(&self) -> Self {
        match self {
            Self::English => Self::French,
            Self::French => Self::English,
        }
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(
            match self {
                Self::English => "EN",
                Self::French => "FR",
            },
            f,
        )
    }
}
