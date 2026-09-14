use sea_query::{
    DeleteStatement, ForeignKeyStatement, IndexStatement, InsertStatement, MysqlQueryBuilder,
    PostgresQueryBuilder, QueryStatementWriter, SchemaStatement, SelectStatement,
    SqliteQueryBuilder, UpdateStatement,
};

/// Run `f` with the SeaQuery builder for `dialect`.
///
/// Both DML (`QueryBuilder`) and DDL (`SchemaBuilder`) builders are the same
/// concrete types per dialect, so this is the single place that maps
/// [`Dialect`] → builder.
macro_rules! with_dialect_builder {
    ($dialect:expr, |$builder:ident| $body:expr) => {
        match $dialect {
            Dialect::Sqlite => {
                let $builder = SqliteQueryBuilder;
                $body
            }
            Dialect::Postgres => {
                let $builder = PostgresQueryBuilder;
                $body
            }
            Dialect::Mysql => {
                let $builder = MysqlQueryBuilder;
                $body
            }
        }
    };
}

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

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy)]
pub enum Dialect {
    Sqlite,
    Postgres,
    Mysql,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f32),
    Text(String),
    Bool(bool),
    Bytes(Vec<u8>),
    Null,
}

macro_rules! try_from_impl_for_value {
    ($($variant:ident => $ty:ty),* $(,)?) => {
        $(
            impl TryFrom<Value> for $ty {
                type Error = Error;
                fn try_from(value: Value) -> Result<$ty> {
                    match value {
                        Value::$variant(x) => Ok(x),
                        _ => Err(Error::TypeMismatch),
                    }
                }
            }
        )*
    };
}

try_from_impl_for_value! {
    Int   => i64,
    Float => f32,
    Text  => String,
    Bool  => bool,
    Bytes => Vec<u8>,
}

impl TryFrom<Value> for i32 {
    type Error = Error;
    fn try_from(value: Value) -> Result<i32> {
        match value {
            Value::Int(x) => i32::try_from(x).map_err(|_| Error::TypeMismatch),
            _ => Err(Error::TypeMismatch),
        }
    }
}

impl Value {
    pub fn to<T: TryFrom<Value, Error = Error>>(self) -> Result<T> {
        T::try_from(self)
    }

    pub fn to_opt<T: TryFrom<Value, Error = Error>>(self) -> Result<Option<T>> {
        match self {
            Value::Null => Ok(None),
            v => v.to().map(Some),
        }
    }
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
            sea_query::Value::Float(Some(v)) => Ok(Value::Float(v)),
            sea_query::Value::Double(Some(v)) => Ok(Value::Float(v as f32)),
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

// Trait for mapping a row back to a struct, used by the database executor trait, must be implemented by the models like this
// impl FromRow for User {
//     fn from_row(row: &impl AnyRow) -> Result<Self> {
//         Ok(Self {
//             id:    row.get("id")?.to()?,
//             email: row.get("email")?.to()?,
//             nick:  row.get("nick")?.to_opt()?,   // Option<String>
//         })
//     }
// }
pub trait FromRow: Sized {
    fn from_row(row: &impl AnyRow) -> Result<Self>;
}

// Must be implemented by the database adapter for the row result type, used by the models
pub trait AnyRow {
    fn get(&self, col: &str) -> Result<Value>;
}

/// Database port. Callers pass SeaQuery statements; adapters render SQL for their engine.
///
/// Futures are not `Send` so Cloudflare D1 can implement this trait.
pub trait DatabaseExecutor {
    fn fetch_one<T: FromRow>(&self, stmt: &SelectStatement) -> impl Future<Output = Result<T>>;

    fn fetch_optional<T: FromRow>(
        &self,
        stmt: &SelectStatement,
    ) -> impl Future<Output = Result<Option<T>>>;

    fn fetch_all<T: FromRow>(&self, stmt: &SelectStatement)
    -> impl Future<Output = Result<Vec<T>>>;

    /// INSERT into an autoincrement table. Always returns the generated primary key.
    fn insert(&self, stmt: &InsertStatement) -> impl Future<Output = Result<i64>>;

    /// UPDATE / DELETE / INSERT where the primary key is already known.
    fn execute(
        &self,
        stmt: &impl ExecutableStatement,
    ) -> impl Future<Output = Result<ExecuteResult>>;
}

/// Render a DML query (`SELECT` / `INSERT` / `UPDATE` / `DELETE`) to SQL + bind values.
#[cfg(any(feature = "native", feature = "cloudflare"))]
pub(crate) fn query_to_sql(
    stmt: &impl QueryStatementWriter,
    dialect: Dialect,
) -> Result<(String, Vec<Value>)> {
    let (sql, values) = with_dialect_builder!(dialect, |builder| stmt.build(builder));
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
    with_dialect_builder!(dialect, |builder| schema_to_sql_with(stmt, builder))
}

fn schema_to_sql_with(stmt: &Statement, builder: impl sea_query::SchemaBuilder) -> String {
    match stmt {
        SchemaStatement::TableStatement(ts) => ts.to_string(builder),
        SchemaStatement::IndexStatement(ix) => match ix {
            IndexStatement::Create(c) => c.to_string(builder),
            IndexStatement::Drop(d) => d.to_string(builder),
            _ => panic!("unsupported index statement variant"),
        },
        SchemaStatement::ForeignKeyStatement(fk) => match fk {
            ForeignKeyStatement::Create(c) => c.to_string(builder),
            ForeignKeyStatement::Drop(d) => d.to_string(builder),
            _ => panic!("unsupported foreign key statement variant"),
        },
        _ => panic!("unsupported schema statement variant"),
    }
}

pub trait Migration {
    fn name(&self) -> &str;
    fn up(&self) -> Vec<Statement>;
    fn down(&self) -> Vec<Statement>;
}
