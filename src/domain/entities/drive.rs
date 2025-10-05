use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Drive {
    pub name: String,
    pub available_space: u64,
}

#[derive(Clone, Debug)]
pub struct DriveToDelete {
    pub name: String,
}

impl Display for Drive {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.name.clone())
    }
}
