use std::collections::HashMap;

use crate::traits::db::{
    AnyRow, DatabaseExecutor, Dialect, Error, ExecutableStatement, ExecuteResult, FromRow, Result,
    Value, render_sql,
};
use sea_query::{InsertStatement, SelectStatement};
use serde_json::Value as JsonValue;
use worker::D1Database;
use worker::wasm_bindgen::JsValue;

pub struct D1Executor {
    db: D1Database,
}

impl D1Executor {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }

    fn sqlite_sql(stmt: &impl sea_query::QueryStatementWriter) -> Result<(String, Vec<Value>)> {
        render_sql(stmt, Dialect::Sqlite)
    }

    fn prepare(&self, sql: &str, values: &[Value]) -> Result<worker::D1PreparedStatement> {
        let js_values: Vec<JsValue> = values.iter().map(value_to_js).collect();
        self.db.prepare(sql).bind(&js_values).map_err(map_d1)
    }
}

fn value_to_js(value: &Value) -> JsValue {
    match value {
        Value::Null => JsValue::NULL,
        Value::Bool(v) => JsValue::from_bool(*v),
        Value::Text(v) => JsValue::from_str(v),
        Value::Int(v) => JsValue::from_f64(*v as f64),
        Value::Float(v) => JsValue::from_f64(f64::from(*v)),
        Value::Bytes(v) => worker::js_sys::Uint8Array::from(v.as_slice()).into(),
    }
}

fn map_d1(err: worker::Error) -> Error {
    map_d1_message(&err.to_string())
}

fn map_d1_message(msg: &str) -> Error {
    if msg
        .to_ascii_lowercase()
        .contains("unique constraint failed")
    {
        Error::Conflict
    } else {
        Error::Backend(msg.to_string())
    }
}

fn json_to_value(value: &JsonValue) -> Result<Value> {
    match value {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(v) => Ok(Value::Bool(*v)),
        JsonValue::String(v) => Ok(Value::Text(v.clone())),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if let Some(u) = n.as_u64() {
                i64::try_from(u)
                    .map(Value::Int)
                    .map_err(|_| Error::TypeMismatch)
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                    Ok(Value::Int(f as i64))
                } else {
                    Ok(Value::Float(f as f32))
                }
            } else {
                Err(Error::TypeMismatch)
            }
        }
        JsonValue::Array(items) => {
            let bytes = items
                .iter()
                .map(|item| {
                    item.as_u64()
                        .and_then(|b| u8::try_from(b).ok())
                        .ok_or(Error::TypeMismatch)
                })
                .collect::<Result<Vec<u8>>>()?;
            Ok(Value::Bytes(bytes))
        }
        JsonValue::Object(_) => Err(Error::TypeMismatch),
    }
}

struct D1AnyRow<'a>(&'a HashMap<String, JsonValue>);

impl AnyRow for D1AnyRow<'_> {
    fn get(&self, col: &str) -> Result<Value> {
        match self.0.get(col) {
            None => Err(Error::Backend(format!("missing column: {col}"))),
            Some(value) => json_to_value(value),
        }
    }
}

impl DatabaseExecutor for D1Executor {
    async fn fetch_one<T: FromRow>(&self, stmt: &SelectStatement) -> Result<T> {
        match self.fetch_optional(stmt).await? {
            Some(row) => Ok(row),
            None => Err(Error::NotFound),
        }
    }

    async fn fetch_optional<T: FromRow>(&self, stmt: &SelectStatement) -> Result<Option<T>> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let row = self
            .prepare(&sql, &values)?
            .first::<HashMap<String, JsonValue>>(None)
            .await
            .map_err(map_d1)?;
        match row {
            Some(row) => T::from_row(&D1AnyRow(&row)).map(Some),
            None => Ok(None),
        }
    }

    async fn fetch_all<T: FromRow>(&self, stmt: &SelectStatement) -> Result<Vec<T>> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let result = self.prepare(&sql, &values)?.all().await.map_err(map_d1)?;
        if !result.success() {
            return Err(map_d1_message(
                &result.error().unwrap_or_else(|| "d1 query failed".into()),
            ));
        }
        let rows: Vec<HashMap<String, JsonValue>> = result.results().map_err(map_d1)?;
        rows.iter().map(|row| T::from_row(&D1AnyRow(row))).collect()
    }

    async fn insert(&self, stmt: &InsertStatement) -> Result<i64> {
        let result = self.run(stmt).await?;
        result
            .meta()
            .map_err(map_d1)?
            .and_then(|meta| meta.last_row_id)
            .ok_or_else(|| Error::Backend("insert did not return a row id".into()))
    }

    async fn execute(&self, stmt: &impl ExecutableStatement) -> Result<ExecuteResult> {
        let result = self.run(stmt).await?;
        let rows_affected = result
            .meta()
            .map_err(map_d1)?
            .and_then(|meta| meta.changes.or(meta.rows_written))
            .unwrap_or(0) as u64;
        Ok(ExecuteResult { rows_affected })
    }
}

impl D1Executor {
    async fn run(&self, stmt: &impl sea_query::QueryStatementWriter) -> Result<worker::D1Result> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        let result = self.prepare(&sql, &values)?.run().await.map_err(map_d1)?;
        if !result.success() {
            return Err(map_d1_message(
                &result.error().unwrap_or_else(|| "d1 query failed".into()),
            ));
        }
        Ok(result)
    }
}
