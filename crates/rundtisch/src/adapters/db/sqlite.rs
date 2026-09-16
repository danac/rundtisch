use crate::traits::db::{
    AnyRow, DatabaseExecutor, Dialect, Error, ExecutableStatement, ExecuteResult, FromRow, Result,
    Value, query_to_sql,
};
use sea_query::{InsertStatement, SelectStatement};
use sqlx::query::Query;
use sqlx::sqlite::{SqliteArguments, SqliteRow};
use sqlx::{Row, Sqlite, TypeInfo, ValueRef};

pub struct SqliteExecutor {
    pool: sqlx::SqlitePool,
}

impl SqliteExecutor {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    fn sqlite_sql(stmt: &impl sea_query::QueryStatementWriter) -> Result<(String, Vec<Value>)> {
        query_to_sql(stmt, Dialect::Sqlite)
    }

    /// Build a sqlx query with all values bound. Borrows from `sql` and `values` to avoid cloning
    /// the values passed by reference.
    fn bind<'q>(&self, sql: &'q str, values: &'q [Value]) -> Query<'q, Sqlite, SqliteArguments> {
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for v in values {
            q = match v {
                Value::Int(x) => q.bind(*x),
                Value::Float(x) => q.bind(*x),
                Value::Text(x) => q.bind(x.as_str()),
                Value::Bool(x) => q.bind(*x),
                Value::Bytes(x) => q.bind(x.as_slice()),
                Value::Null => q.bind(Option::<i64>::None),
            };
        }
        q
    }
}

fn map_sqlx(err: sqlx::Error) -> Error {
    match err {
        sqlx::Error::RowNotFound => Error::NotFound,
        sqlx::Error::Database(ref db) if db.is_unique_violation() => Error::Conflict,
        other => Error::Backend(other.to_string()),
    }
}

/// Adapts a sqlx `SqliteRow` to the generic `AnyRow` trait.
struct SqliteAnyRow<'a>(&'a SqliteRow);

impl AnyRow for SqliteAnyRow<'_> {
    fn get(&self, col: &str) -> Result<Value> {
        // Grab the raw value so we can inspect NULL and type info without
        // guessing through a chain of `try_get::<T, _>` fallbacks.
        let raw = self
            .0
            .try_get_raw(col)
            .map_err(|e| Error::Backend(e.to_string()))?;

        if raw.is_null() {
            return Ok(Value::Null);
        }

        // SQLite storage classes: INTEGER, REAL, TEXT, BLOB, NULL.
        // Booleans are stored as INTEGER; callers who want a bool should
        // query INTEGER and convert, unless the declared type is BOOLEAN.
        let type_info = raw.type_info();
        match type_info.name() {
            "INTEGER" | "INT" | "BIGINT" | "INT8" => {
                let v: i64 = self
                    .0
                    .try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Int(v))
            }
            "REAL" | "FLOAT" | "DOUBLE" => {
                let v: f64 = self
                    .0
                    .try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Float(v))
            }
            "TEXT" | "VARCHAR" => {
                let v: String = self
                    .0
                    .try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Text(v))
            }
            "BLOB" => {
                let v: Vec<u8> = self
                    .0
                    .try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Bytes(v))
            }
            "BOOLEAN" => {
                let v: bool = self
                    .0
                    .try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Bool(v))
            }
            _ => Err(Error::TypeMismatch),
        }
    }
}

impl DatabaseExecutor for SqliteExecutor {
    async fn fetch_one<T: FromRow>(&self, stmt: &SelectStatement) -> Result<T> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let row = self
            .bind(&sql, &values)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx)?;
        T::from_row(&SqliteAnyRow(&row))
    }

    async fn fetch_optional<T: FromRow>(&self, stmt: &SelectStatement) -> Result<Option<T>> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let row = self
            .bind(&sql, &values)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        match row {
            Some(row) => T::from_row(&SqliteAnyRow(&row)).map(Some),
            None => Ok(None),
        }
    }

    async fn fetch_all<T: FromRow>(&self, stmt: &SelectStatement) -> Result<Vec<T>> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let rows = self
            .bind(&sql, &values)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
        rows.iter().map(|r| T::from_row(&SqliteAnyRow(r))).collect()
    }

    async fn insert(&self, stmt: &InsertStatement) -> Result<i64> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let res = self
            .bind(&sql, &values)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(res.last_insert_rowid())
    }

    async fn execute(&self, stmt: &impl ExecutableStatement) -> Result<ExecuteResult> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let res = self
            .bind(&sql, &values)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(ExecuteResult {
            rows_affected: res.rows_affected(),
        })
    }
}
