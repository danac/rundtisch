use crate::traits::db::{AnyRow, DatabaseExecutor, Dialect, Error, FromRow, Result, Value};
use sqlx::{Row, TypeInfo, ValueRef, Sqlite, query::Query, sqlite::{SqliteArguments, SqliteRow}};

pub struct SqliteExecutor {
    pool: sqlx::SqlitePool,
}

impl SqliteExecutor {
    /// Build a sqlx query with all values bound. Borrows from `sql` and `values` to avoid cloning
    /// the values passed by reference.
    fn build(&self, sql: &str, values: &[Value]) -> Query<'_, Sqlite, SqliteArguments> {
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for v in values {
            q = match v {
                Value::Int(x) => q.bind(*x),
                Value::Float(x) => q.bind(*x),
                Value::Text(x) => q.bind(x.as_str()),
                Value::Bool(x) => q.bind(*x),
                Value::Bytes(x) => q.bind(x.as_slice()),
                Value::Null => q.bind(Option::<i32>::None),
            };
        }
        q
    }
}

/// Adapts a sqlx `SqliteRow` to the generic `AnyRow` trait.
struct SqliteAnyRow<'a>(&'a SqliteRow);

impl<'a> AnyRow for SqliteAnyRow<'a> {
    fn get(&self, col: &str) -> Result<Value> {
        // Grab the raw value so we can inspect NULL and type info without
        // guessing through a chain of `try_get::<T, _>` fallbacks.
        let raw = self.0
            .try_get_raw(col)
            .map_err(|e| Error::Backend(e.to_string()))?;

        if raw.is_null() {
            return Ok(Value::Null);
        }

        // SQLite storage classes: INTEGER, REAL, TEXT, BLOB, NULL.
        // We map them to our `Value` enum. Booleans are stored as INTEGER,
        // so callers who want a bool should query it as Int and convert,
        // or you can add a dedicated schema hint later.
        let type_info = raw.type_info();
        match type_info.name() {
            "INTEGER" | "INT" | "BIGINT" | "INT8" => {
                let v: i64 = self.0.try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Int(v as i32))
            }
            "REAL" | "FLOAT" | "DOUBLE" => {
                let v: f64 = self.0.try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Float(v as f32))
            }
            "TEXT" | "VARCHAR" => {
                let v: String = self.0.try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Text(v))
            }
            "BLOB" => {
                let v: Vec<u8> = self.0.try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Bytes(v))
            }
            "BOOLEAN" => {
                let v: bool = self.0.try_get(col)
                    .map_err(|e| Error::Backend(e.to_string()))?;
                Ok(Value::Bool(v))
            }
            _ => Err(Error::TypeMismatch),
        }
    }
}

impl DatabaseExecutor for SqliteExecutor {
    async fn fetch_one<T: FromRow>(&self, sql: &str, values: &[Value]) -> Result<T> {
        let row = self
            .build(sql, values)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => Error::NotFound,
                other => Error::Backend(other.to_string()),
            })?;
        T::from_row(&SqliteAnyRow(&row))
    }

    async fn fetch_all<T: FromRow>(&self, sql: &str, values: &[Value]) -> Result<Vec<T>> {
        let rows = self
            .build(sql, values)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Backend(e.to_string()))?;
        rows.iter()
            .map(|r| T::from_row(&SqliteAnyRow(r)))
            .collect()
    }

    async fn execute(&self, sql: &str, values: &[Value]) -> Result<usize> {
        let res = self
            .build(sql, values)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Backend(e.to_string()))?;
        Ok(res.rows_affected() as usize)
    }

    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
}