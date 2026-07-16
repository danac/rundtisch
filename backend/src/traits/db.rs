use sea_query::{ForeignKeyStatement, IndexStatement, SchemaStatement};

#[derive(Debug)]
pub enum Error {
    NotFound,
    TypeMismatch,
    Backend(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy)]
pub enum Dialect {
    Sqlite,
    Postgres,
    Mysql,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i32),
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
    Int   => i32,
    Float => f32,
    Text  => String,
    Bool  => bool,
    Bytes => Vec<u8>,
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

// Must be implemented by the database adapter
pub trait DatabaseExecutor {
    // Prefer not to use the async keyword in the public trait definition
    // Execute a query that returns rows
    fn fetch_one<T: FromRow>(&self, sql: &str, values: &[Value]) -> impl Future<Output = Result<T>>;
    fn fetch_all<T: FromRow>(
        &self,
        sql: &str,
        values: &[Value],
    ) -> impl Future<Output = Result<Vec<T>>>;

    // Execute a mutation, returns the number of affected rows
    fn execute(&self, sql: &str, values: &[Value]) -> impl Future<Output = Result<usize>>;

    // Which dialect to use when building queries
    fn dialect(&self) -> Dialect;
}

pub type Statement = SchemaStatement;

pub fn statement_to_sql(stmt: &Statement, dialect: Dialect) -> String {
    match dialect {
        Dialect::Sqlite => statement_to_sql_with(stmt, sea_query::SqliteQueryBuilder),
        Dialect::Postgres => statement_to_sql_with(stmt, sea_query::PostgresQueryBuilder),
        Dialect::Mysql => statement_to_sql_with(stmt, sea_query::MysqlQueryBuilder),
    }
}

fn statement_to_sql_with<B>(stmt: &Statement, builder: B) -> String
where
    B: sea_query::SchemaBuilder,
{
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
