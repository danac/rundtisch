use crate::adapters::db::row_de::{self, RowCells};
use crate::traits::db::{
    DatabaseExecutor, DbRecord, Dialect, Error, ExecutableStatement, ExecuteResult, Result, Value,
    query_to_sql,
};
use sea_query::{InsertStatement, SelectStatement};
use sqlx::query::Query;
use sqlx::sqlite::{SqliteArguments, SqliteRow};
use sqlx::{Row, Sqlite, TypeInfo, ValueRef};
use std::path::Path;
use crate::traits::db::{AnyRow, DatabaseExecutor, Dialect, Error, FromRow, Result, Value};
use sqlx::{
    Row, Sqlite, TypeInfo, ValueRef,
    query::Query,
    sqlite::{SqliteArguments, SqliteRow},
};

pub struct SqliteExecutor {
    pool: sqlx::SqlitePool,
}

impl SqliteExecutor {
 
    pub async fn new(database_path: impl AsRef<Path>) -> Self {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);
        let pool = sqlx::SqlitePool::connect_with(options)
            .await
            .expect("failed to open SQLite database");
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

struct SqliteCells<'a>(&'a SqliteRow);

impl SqliteCells<'_> {
    fn raw<'r>(&'r self, col: &str) -> Result<sqlx::sqlite::SqliteValueRef<'r>> {
        self.0
            .try_get_raw(col)
            .map_err(|_| Error::Backend(format!("missing column: {col}")))
    }

    fn type_name(&self, col: &str) -> Result<String> {
        Ok(self.raw(col)?.type_info().name().to_string())
    }

    /// Decode after the storage class has already been checked. sqlx `try_get`
    /// would reject INTEGER→`f64` and BOOLEAN→`i64` as incompatible.
    fn decode<'r, T>(&'r self, col: &str) -> Result<T>
    where
        T: sqlx::decode::Decode<'r, Sqlite>,
    {
        self.0
            .try_get_unchecked(col)
            .map_err(|e| Error::Backend(e.to_string()))
    }
}

impl RowCells for SqliteCells<'_> {
    fn is_null(&self, col: &str) -> Result<bool> {
        Ok(self.raw(col)?.is_null())
    }

    fn get_bool(&self, col: &str) -> Result<bool> {
        if self.is_null(col)? {
            return Err(Error::TypeMismatch);
        }
        // sqlx TypeInfo::name() for these affinities is INTEGER or BOOLEAN.
        match self.type_name(col)?.as_str() {
            "BOOLEAN" | "INTEGER" => row_de::i64_as_bool(self.decode(col)?),
            _ => Err(Error::TypeMismatch),
        }
    }

    fn get_i64(&self, col: &str) -> Result<i64> {
        if self.is_null(col)? {
            return Err(Error::TypeMismatch);
        }
        match self.type_name(col)?.as_str() {
            "INTEGER" | "BOOLEAN" => self.decode(col),
            "REAL" => row_de::integer_valued_f64(self.decode(col)?),
            _ => Err(Error::TypeMismatch),
        }
    }

    fn get_f64(&self, col: &str) -> Result<f64> {
        if self.is_null(col)? {
            return Err(Error::TypeMismatch);
        }
        match self.type_name(col)?.as_str() {
            "REAL" => self.decode(col),
            "INTEGER" => Ok(self.decode::<i64>(col)? as f64),
            _ => Err(Error::TypeMismatch),
        }
    }

    fn get_string(&self, col: &str) -> Result<String> {
        if self.is_null(col)? {
            return Err(Error::TypeMismatch);
        }
        match self.type_name(col)?.as_str() {
            "TEXT" => self.decode(col),
            _ => Err(Error::TypeMismatch),
        }
    }

    fn get_bytes(&self, col: &str) -> Result<Vec<u8>> {
        if self.is_null(col)? {
            return Err(Error::TypeMismatch);
        }
        match self.type_name(col)?.as_str() {
            "BLOB" => self.decode(col),
            _ => Err(Error::TypeMismatch),
        }
    }
}

impl DatabaseExecutor for SqliteExecutor {
    async fn fetch_one<T: DbRecord>(&self, stmt: &SelectStatement) -> Result<T> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let row = self
            .bind(&sql, &values)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx)?;
        row_de::from_row(&SqliteCells(&row))
    }

    async fn fetch_optional<T: DbRecord>(&self, stmt: &SelectStatement) -> Result<Option<T>> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let row = self
            .bind(&sql, &values)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;
        match row {
            Some(row) => row_de::from_row(&SqliteCells(&row)).map(Some),
            None => Ok(None),
        }
    }

    async fn fetch_all<T: DbRecord>(&self, stmt: &SelectStatement) -> Result<Vec<T>> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let rows = self
            .bind(&sql, &values)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;
        rows.iter()
            .map(|r| row_de::from_row(&SqliteCells(r)))
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use sea_query::{Alias, Expr, ExprTrait, Query};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Item {
        id: i64,
        score: f64,
        flag: bool,
        name: Option<String>,
        blob: Vec<u8>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct ScoreAsInt {
        score: i64,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct IdAsFloat {
        id: f64,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Kind {
        Admin,
        User,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Typed {
        id: i64,
        flag: bool,
        kind: Kind,
        whole: i64,
    }

    async fn executor() -> SqliteExecutor {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        sqlx::query(
            "CREATE TABLE item (
                id INTEGER PRIMARY KEY,
                score REAL NOT NULL,
                flag INTEGER NOT NULL,
                name TEXT,
                blob BLOB NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create table");
        sqlx::query(
            "CREATE TABLE typed (
                id INTEGER PRIMARY KEY,
                flag BOOLEAN NOT NULL,
                kind TEXT NOT NULL,
                whole REAL NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create typed table");
        SqliteExecutor::new(pool)
    }

    fn select_item(id: i64) -> SelectStatement {
        Query::select()
            .columns([
                Alias::new("id"),
                Alias::new("score"),
                Alias::new("flag"),
                Alias::new("name"),
                Alias::new("blob"),
            ])
            .from(Alias::new("item"))
            .and_where(Expr::col(Alias::new("id")).eq(Expr::val(id)))
            .to_owned()
    }

    fn select_typed(id: i64) -> SelectStatement {
        Query::select()
            .columns([
                Alias::new("id"),
                Alias::new("flag"),
                Alias::new("kind"),
                Alias::new("whole"),
            ])
            .from(Alias::new("typed"))
            .and_where(Expr::col(Alias::new("id")).eq(Expr::val(id)))
            .to_owned()
    }

    #[tokio::test]
    async fn fetch_round_trip_coercions() {
        let exec = executor().await;
        let insert = Query::insert()
            .into_table(Alias::new("item"))
            .columns([
                Alias::new("score"),
                Alias::new("flag"),
                Alias::new("name"),
                Alias::new("blob"),
            ])
            .values_panic([
                1.0.into(),
                true.into(),
                Option::<String>::None.into(),
                vec![1u8, 2, 3].into(),
            ])
            .to_owned();

        let id = exec.insert(&insert).await.expect("insert");
        let row: Item = exec.fetch_one(&select_item(id)).await.expect("fetch");
        assert_eq!(
            row,
            Item {
                id,
                score: 1.0,
                flag: true,
                name: None,
                blob: vec![1, 2, 3],
            }
        );
    }

    #[tokio::test]
    async fn fetch_optional_none() {
        let exec = executor().await;
        let row: Option<Item> = exec
            .fetch_optional(&select_item(99))
            .await
            .expect("fetch_optional");
        assert_eq!(row, None);
    }

    #[tokio::test]
    async fn integer_flag_zero_is_false() {
        let exec = executor().await;
        let insert = Query::insert()
            .into_table(Alias::new("item"))
            .columns([
                Alias::new("score"),
                Alias::new("flag"),
                Alias::new("name"),
                Alias::new("blob"),
            ])
            .values_panic([
                2.5.into(),
                false.into(),
                "n".into(),
                Vec::<u8>::new().into(),
            ])
            .to_owned();
        let id = exec.insert(&insert).await.expect("insert");
        let row: Item = exec.fetch_one(&select_item(id)).await.expect("fetch");
        assert_eq!(row.flag, false);
        assert_eq!(row.score, 2.5);
        assert_eq!(row.name.as_deref(), Some("n"));
        assert_eq!(row.blob, Vec::<u8>::new());
    }

    #[tokio::test]
    async fn integer_pk_decodes_as_f64_and_whole_real_as_i64() {
        let exec = executor().await;
        let insert = Query::insert()
            .into_table(Alias::new("item"))
            .columns([
                Alias::new("score"),
                Alias::new("flag"),
                Alias::new("name"),
                Alias::new("blob"),
            ])
            .values_panic([
                4.0.into(),
                true.into(),
                Option::<String>::None.into(),
                vec![9u8].into(),
            ])
            .to_owned();
        let id = exec.insert(&insert).await.expect("insert");

        let as_float: IdAsFloat = exec.fetch_one(&select_item(id)).await.expect("id as f64");
        assert_eq!(as_float.id, id as f64);

        let as_int: ScoreAsInt = exec
            .fetch_one(&select_item(id))
            .await
            .expect("whole REAL as i64");
        assert_eq!(as_int.score, 4);
    }

    #[tokio::test]
    async fn fractional_real_is_not_an_integer() {
        let exec = executor().await;
        let insert = Query::insert()
            .into_table(Alias::new("item"))
            .columns([
                Alias::new("score"),
                Alias::new("flag"),
                Alias::new("name"),
                Alias::new("blob"),
            ])
            .values_panic([
                2.5.into(),
                true.into(),
                Option::<String>::None.into(),
                vec![0u8].into(),
            ])
            .to_owned();
        let id = exec.insert(&insert).await.expect("insert");
        let err = exec
            .fetch_one::<ScoreAsInt>(&select_item(id))
            .await
            .expect_err("2.5 is not an i64");
        assert_eq!(err, Error::TypeMismatch);
    }

    #[tokio::test]
    async fn fetch_one_missing_row_is_not_found() {
        let exec = executor().await;
        let err = exec
            .fetch_one::<Item>(&select_item(99))
            .await
            .expect_err("missing row");
        assert_eq!(err, Error::NotFound);
    }

    #[tokio::test]
    async fn fetch_all_and_boolean_text_real_coercions() {
        let exec = executor().await;
        let insert = Query::insert()
            .into_table(Alias::new("typed"))
            .columns([Alias::new("flag"), Alias::new("kind"), Alias::new("whole")])
            .values_panic([true.into(), "Admin".into(), 3.0.into()])
            .to_owned();
        let id = exec.insert(&insert).await.expect("insert");
        let rows: Vec<Typed> = exec
            .fetch_all(
                &Query::select()
                    .columns([
                        Alias::new("id"),
                        Alias::new("flag"),
                        Alias::new("kind"),
                        Alias::new("whole"),
                    ])
                    .from(Alias::new("typed"))
                    .to_owned(),
            )
            .await
            .expect("fetch_all");
        assert_eq!(
            rows,
            vec![Typed {
                id,
                flag: true,
                kind: Kind::Admin,
                whole: 3,
            }]
        );

        let one: Typed = exec.fetch_one(&select_typed(id)).await.expect("fetch");
        assert_eq!(one.kind, Kind::Admin);
        assert_eq!(one.flag, true);
        assert_eq!(one.whole, 3);
    }

    #[tokio::test]
    async fn integer_flag_two_is_not_bool() {
        let exec = executor().await;
        sqlx::query("INSERT INTO item (score, flag, name, blob) VALUES (1.0, 2, NULL, x'00')")
            .execute(&exec.pool)
            .await
            .expect("insert flag=2");
        let err = exec
            .fetch_one::<Item>(
                &Query::select()
                    .columns([
                        Alias::new("id"),
                        Alias::new("score"),
                        Alias::new("flag"),
                        Alias::new("name"),
                        Alias::new("blob"),
                    ])
                    .from(Alias::new("item"))
                    .to_owned(),
            )
            .await
            .expect_err("flag=2");
        assert_eq!(err, Error::TypeMismatch);
    }
}
