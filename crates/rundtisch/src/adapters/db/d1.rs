use crate::traits::db::{
    AnyRow, DatabaseExecutor, Dialect, Error, ExecutableStatement, ExecuteResult, FromRow, Result,
    Value, query_to_sql,
};
use sea_query::{InsertStatement, SelectStatement};
use worker::D1Database;
use worker::js_sys::{Array, ArrayBuffer, Number, Object, Reflect, Uint8Array};
use worker::wasm_bindgen::JsCast;
use worker::wasm_bindgen::JsValue;
use worker::wasm_bindgen_futures::JsFuture;

pub struct D1Executor {
    db: D1Database,
}

impl D1Executor {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }

    fn sqlite_sql(stmt: &impl sea_query::QueryStatementWriter) -> Result<(String, Vec<Value>)> {
        query_to_sql(stmt, Dialect::Sqlite)
    }

    fn prepare(&self, sql: &str, values: &[Value]) -> Result<worker::D1PreparedStatement> {
        let js_values: Vec<JsValue> = values.iter().map(value_to_js).collect();
        self.db.prepare(sql).bind(&js_values).map_err(map_d1)
    }

    async fn first_row(&self, sql: &str, values: &[Value]) -> Result<Option<Object>> {
        let stmt = self.prepare(sql, values)?;
        let js = JsFuture::from(stmt.inner().first(None).map_err(map_js)?)
            .await
            .map_err(map_js)?;
        if js.is_null() || js.is_undefined() {
            Ok(None)
        } else {
            js.dyn_into::<Object>()
                .map(Some)
                .map_err(|_| Error::TypeMismatch)
        }
    }

    async fn all_rows(&self, sql: &str, values: &[Value]) -> Result<Vec<Object>> {
        let stmt = self.prepare(sql, values)?;
        let js = JsFuture::from(stmt.inner().all().map_err(map_js)?)
            .await
            .map_err(map_js)?;
        let result = js.dyn_into::<Object>().map_err(|_| Error::TypeMismatch)?;
        d1_result_rows(&result)
    }
}

fn value_to_js(value: &Value) -> JsValue {
    match value {
        Value::Null => JsValue::NULL,
        Value::Bool(v) => JsValue::from_bool(*v),
        Value::Text(v) => JsValue::from_str(v),
        Value::Int(v) => JsValue::from_f64(*v as f64),
        Value::Float(v) => JsValue::from_f64(*v),
        Value::Bytes(v) => Uint8Array::from(v.as_slice()).into(),
    }
}

fn map_d1(err: worker::Error) -> Error {
    map_d1_message(&err.to_string())
}

fn map_js(err: JsValue) -> Error {
    map_d1(err.into())
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

fn d1_result_rows(result: &Object) -> Result<Vec<Object>> {
    let success = Reflect::get(result, &JsValue::from_str("success")).map_err(map_js)?;
    if success.as_bool() != Some(true) {
        let msg = Reflect::get(result, &JsValue::from_str("error"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "d1 query failed".into());
        return Err(map_d1_message(&msg));
    }
    let results = Reflect::get(result, &JsValue::from_str("results")).map_err(map_js)?;
    if results.is_null() || results.is_undefined() {
        return Ok(Vec::new());
    }
    let rows = Array::from(&results);
    let mut out = Vec::with_capacity(rows.length() as usize);
    for value in rows.iter() {
        out.push(
            value
                .dyn_into::<Object>()
                .map_err(|_| Error::TypeMismatch)?,
        );
    }
    Ok(out)
}

/// Map a D1 cell from the original JS value (not JSON).
///
/// D1's binding types: null, boolean (rare on read), string, number, ArrayBuffer /
/// Uint8Array, or a number array for BLOBs. INTEGER and REAL both come back as JS
/// numbers; `Number.isInteger` is the only distinction available.
fn js_to_value(value: &JsValue) -> Result<Value> {
    if value.is_null() || value.is_undefined() {
        return Ok(Value::Null);
    }
    if let Some(v) = value.as_bool() {
        return Ok(Value::Bool(v));
    }
    if let Some(v) = value.as_string() {
        return Ok(Value::Text(v));
    }
    if Number::is_integer(value) {
        let n = value.as_f64().ok_or(Error::TypeMismatch)?;
        if n < i64::MIN as f64 || n > i64::MAX as f64 {
            return Err(Error::TypeMismatch);
        }
        return Ok(Value::Int(n as i64));
    }
    if let Some(n) = value.as_f64() {
        return Ok(Value::Float(n));
    }
    if let Some(buf) = value.dyn_ref::<ArrayBuffer>() {
        return Ok(Value::Bytes(Uint8Array::new(buf).to_vec()));
    }
    if let Some(bytes) = value.dyn_ref::<Uint8Array>() {
        return Ok(Value::Bytes(bytes.to_vec()));
    }
    if Array::is_array(value) {
        return array_to_bytes(&Array::from(value));
    }
    Err(Error::TypeMismatch)
}

fn array_to_bytes(items: &Array) -> Result<Value> {
    let mut bytes = Vec::with_capacity(items.length() as usize);
    for item in items.iter() {
        let n = item.as_f64().ok_or(Error::TypeMismatch)?;
        if n.fract() != 0.0 || !(0.0..=255.0).contains(&n) {
            return Err(Error::TypeMismatch);
        }
        bytes.push(n as u8);
    }
    Ok(Value::Bytes(bytes))
}

struct D1AnyRow<'a>(&'a Object);

impl AnyRow for D1AnyRow<'_> {
    fn get(&self, col: &str) -> Result<Value> {
        let key = JsValue::from_str(col);
        if !Reflect::has(self.0.as_ref(), &key).map_err(map_js)? {
            return Err(Error::Backend(format!("missing column: {col}")));
        }
        let value = Reflect::get(self.0.as_ref(), &key).map_err(map_js)?;
        js_to_value(&value)
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
        match self.first_row(&sql, &values).await? {
            Some(row) => T::from_row(&D1AnyRow(&row)).map(Some),
            None => Ok(None),
        }
    }

    async fn fetch_all<T: FromRow>(&self, stmt: &SelectStatement) -> Result<Vec<T>> {
        let (sql, values) = Self::sqlite_sql(stmt)?;
        self.all_rows(&sql, &values)
            .await?
            .iter()
            .map(|row| T::from_row(&D1AnyRow(row)))
            .collect()
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
