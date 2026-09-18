use sea_query::{
    DeleteStatement, ForeignKeyStatement, IndexStatement, InsertStatement, MysqlQueryBuilder,
    PostgresQueryBuilder, QueryStatementWriter, SchemaStatement, SelectStatement,
    SqliteQueryBuilder, UpdateStatement,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotFound,
    Conflict,
    TypeMismatch,
    Backend(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "not found"),
            Error::Conflict => write!(f, "conflict"),
            Error::TypeMismatch => write!(f, "type mismatch"),
            Error::Backend(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error::Backend(msg.to_string())
    }

    fn invalid_type(_unexp: serde::de::Unexpected, _exp: &dyn serde::de::Expected) -> Self {
        Error::TypeMismatch
    }

    fn invalid_value(_unexp: serde::de::Unexpected, _exp: &dyn serde::de::Expected) -> Self {
        Error::TypeMismatch
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy)]
pub enum Dialect {
    Sqlite,
    Postgres,
    Mysql,
}

/// Bind value produced when adapters render SeaQuery DML.
///
/// Row decoding no longer uses this enum. Fetch maps directly into a
/// [`DbRecord`] with knowledge of the requested Rust type.
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Bytes(Vec<u8>),
    Null,
}

impl TryFrom<sea_query::Value> for Value {
    type Error = Error;

    fn try_from(value: sea_query::Value) -> Result<Value> {
        match value {
            sea_query::Value::Bool(None)
            | sea_query::Value::TinyInt(None)
            | sea_query::Value::SmallInt(None)
            | sea_query::Value::Int(None)
            | sea_query::Value::BigInt(None)
            | sea_query::Value::TinyUnsigned(None)
            | sea_query::Value::SmallUnsigned(None)
            | sea_query::Value::Unsigned(None)
            | sea_query::Value::BigUnsigned(None)
            | sea_query::Value::Float(None)
            | sea_query::Value::Double(None)
            | sea_query::Value::String(None)
            | sea_query::Value::Char(None)
            | sea_query::Value::Bytes(None) => Ok(Value::Null),
            sea_query::Value::Bool(Some(v)) => Ok(Value::Bool(v)),
            sea_query::Value::TinyInt(Some(v)) => Ok(Value::Int(i64::from(v))),
            sea_query::Value::SmallInt(Some(v)) => Ok(Value::Int(i64::from(v))),
            sea_query::Value::Int(Some(v)) => Ok(Value::Int(i64::from(v))),
            sea_query::Value::BigInt(Some(v)) => Ok(Value::Int(v)),
            sea_query::Value::TinyUnsigned(Some(v)) => Ok(Value::Int(i64::from(v))),
            sea_query::Value::SmallUnsigned(Some(v)) => Ok(Value::Int(i64::from(v))),
            sea_query::Value::Unsigned(Some(v)) => Ok(Value::Int(i64::from(v))),
            sea_query::Value::BigUnsigned(Some(v)) => i64::try_from(v)
                .map(Value::Int)
                .map_err(|_| Error::TypeMismatch),
            sea_query::Value::Float(Some(v)) => Ok(Value::Float(f64::from(v))),
            sea_query::Value::Double(Some(v)) => Ok(Value::Float(v)),
            sea_query::Value::String(Some(v)) => Ok(Value::Text(v)),
            sea_query::Value::Char(Some(v)) => Ok(Value::Text(v.to_string())),
            sea_query::Value::Bytes(Some(v)) => Ok(Value::Bytes(v)),
            sea_query::Value::Enum(sea_query::OptionEnum::None(_)) => Ok(Value::Null),
            sea_query::Value::Enum(sea_query::OptionEnum::Some(v)) => {
                Ok(Value::Text(v.value.into_owned()))
            }
        }
    }
}

/// INSERT / UPDATE / DELETE SeaQuery statements. Adapters render these; callers do not.
pub trait ExecutableStatement: QueryStatementWriter {}

impl ExecutableStatement for InsertStatement {}
impl ExecutableStatement for UpdateStatement {}
impl ExecutableStatement for DeleteStatement {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecuteResult {
    pub rows_affected: u64,
}

/// A model that can be stored in SQLite and D1 with the same field behavior.
///
/// Derive both traits:
///
/// ```ignore
/// #[derive(Serialize, Deserialize)]
/// struct User {
///     id: i64,
///     email: String,
///     nick: Option<String>,
/// }
/// ```
///
/// Field names must match selected column names (or use `#[serde(rename)]`).
///
/// Adapters decode **from the requested Rust type**, not from SQLite storage
/// class / JS typeof, so the same struct works on both backends:
///
/// | Field type | SQLite | D1 |
/// |---|---|---|
/// | `i64` / `i32` | INTEGER, or REAL with a 0 fractional part | JS number that is an integer |
/// | `f64` / `f32` | REAL or INTEGER | any JS number |
/// | `bool` | BOOLEAN, or INTEGER `0`/`1` | JS boolean, or number `0`/`1` |
/// | `String` | TEXT | JS string |
/// | `Vec<u8>` | BLOB | `ArrayBuffer`, `Uint8Array`, or number array |
/// | `Option<T>` | NULL | `null` / `undefined` |
/// | unit enum | TEXT | JS string |
///
/// Documented adapter differences that do **not** change a given model's
/// behavior when the schema matches the field types above:
///
/// - D1 integers travel as JS numbers. Values outside `Number.MAX_SAFE_INTEGER`
///   (`2^53 - 1`) cannot round-trip. SQLite INTEGER is a full `i64`.
/// - D1 has no separate INTEGER vs REAL at rest; both are JS `Number`.
///   Requested `f64` vs `i64` still selects the conversion.
pub trait DbRecord: Serialize + DeserializeOwned {}

impl<T: Serialize + DeserializeOwned> DbRecord for T {}

/// Database port. Callers pass SeaQuery statements; adapters render SQL for their engine.
///
/// Fetch deserializes each row into `T: `[`DbRecord`]. Futures are not `Send`
/// so Cloudflare D1 can implement this trait.
pub trait DatabaseExecutor {
    fn fetch_one<T: DbRecord>(&self, stmt: &SelectStatement) -> impl Future<Output = Result<T>>;

    fn fetch_optional<T: DbRecord>(
        &self,
        stmt: &SelectStatement,
    ) -> impl Future<Output = Result<Option<T>>>;

    fn fetch_all<T: DbRecord>(
        &self,
        stmt: &SelectStatement,
    ) -> impl Future<Output = Result<Vec<T>>>;

    /// INSERT into an autoincrement table. Always returns the generated primary key.
    fn insert(&self, stmt: &InsertStatement) -> impl Future<Output = Result<i64>>;

    /// UPDATE / DELETE / INSERT where the primary key is already known.
    fn execute(
        &self,
        stmt: &impl ExecutableStatement,
    ) -> impl Future<Output = Result<ExecuteResult>>;
}

/// Render a DML query (`SELECT` / `INSERT` / `UPDATE` / `DELETE`) to SQL + bind values.
pub(crate) fn query_to_sql(
    stmt: &impl QueryStatementWriter,
    dialect: Dialect,
) -> Result<(String, Vec<Value>)> {
    let (sql, values) = match dialect {
        Dialect::Sqlite => stmt.build(SqliteQueryBuilder),
        Dialect::Postgres => stmt.build(PostgresQueryBuilder),
        Dialect::Mysql => stmt.build(MysqlQueryBuilder),
    };
    let values = values
        .0
        .into_iter()
        .map(Value::try_from)
        .collect::<Result<Vec<_>>>()?;
    Ok((sql, values))
}

pub type Statement = SchemaStatement;

/// Render a DDL / schema statement (`CREATE` / `DROP` table, index, FK) to SQL.
///
/// Schema SQL has no bind parameters.
pub fn schema_to_sql(stmt: &Statement, dialect: Dialect) -> String {
    match dialect {
        Dialect::Sqlite => schema_to_sql_with(stmt, SqliteQueryBuilder),
        Dialect::Postgres => schema_to_sql_with(stmt, PostgresQueryBuilder),
        Dialect::Mysql => schema_to_sql_with(stmt, MysqlQueryBuilder),
    }
}

fn schema_to_sql_with(stmt: &Statement, builder: impl sea_query::SchemaBuilder) -> String {
    match stmt {
        SchemaStatement::TableStatement(ts) => ts.build(builder),
        SchemaStatement::IndexStatement(IndexStatement::Create(s)) => s.build(builder),
        SchemaStatement::IndexStatement(IndexStatement::Drop(s)) => s.build(builder),
        SchemaStatement::ForeignKeyStatement(ForeignKeyStatement::Create(s)) => s.build(builder),
        SchemaStatement::ForeignKeyStatement(ForeignKeyStatement::Drop(s)) => s.build(builder),
        _ => panic!("unsupported schema statement variant"),
    }
}

pub trait Migration {
    fn name(&self) -> &str;
    fn up(&self) -> Vec<Statement>;
    fn down(&self) -> Vec<Statement>;
}
