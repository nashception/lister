#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytes(pub i64);

impl From<i64> for Bytes {
    fn from(value: i64) -> Self {
        Bytes(value)
    }
}

impl From<Bytes> for i64 {
    fn from(value: Bytes) -> i64 {
        value.0
    }
}

impl From<u64> for Bytes {
    fn from(value: u64) -> Self {
        Bytes(value as i64)
    }
}

impl Bytes {
    pub fn as_u64(&self) -> u64 {
        self.0 as u64
    }

    pub fn as_i64(&self) -> i64 {
        self.0
    }
}
