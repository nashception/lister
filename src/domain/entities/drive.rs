use std::fmt::{Display, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct Drive {
    pub name: String,
}

impl Display for Drive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
        write!(f, "{}", self.name.clone())
    }
}
