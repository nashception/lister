pub trait ToI64 {
    fn to_i64_or_zero(self) -> i64;
}

pub trait ToU64 {
    fn to_u64_or_zero(self) -> u64;
}

impl ToI64 for u64 {
    fn to_i64_or_zero(self) -> i64 {
        i64::try_from(self).unwrap_or(0)
    }
}

impl ToU64 for i64 {
    fn to_u64_or_zero(self) -> u64 {
        u64::try_from(self).unwrap_or(0)
    }
}
