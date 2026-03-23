use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Binary;
use diesel::sqlite::Sqlite;
use uuid::Uuid;

#[derive(AsExpression, FromSqlRow)]
#[diesel(sql_type = Binary)]
#[derive(Clone, Copy, Debug)]
pub struct UuidSqlite(pub Uuid);

impl UuidSqlite {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl ToSql<Binary, Sqlite> for UuidSqlite {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <[u8] as ToSql<Binary, Sqlite>>::to_sql(self.0.as_bytes(), out)
    }
}

impl FromSql<Binary, Sqlite> for UuidSqlite {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let bytes = <Vec<u8> as FromSql<Binary, Sqlite>>::from_sql(bytes)?;
        Uuid::from_slice(&bytes)
            .map(UuidSqlite)
            .map_err(Into::into)
    }
}
