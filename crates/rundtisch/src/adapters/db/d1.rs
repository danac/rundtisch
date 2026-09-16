//! Cloudflare D1 adapter (query methods not yet implemented).

use crate::traits::db::{DatabaseExecutor, Dialect, FromRow, Result, Value};
use worker::D1Database;

pub struct D1Executor {
    #[allow(dead_code)]
    db: D1Database,
}

impl D1Executor {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

impl DatabaseExecutor for D1Executor {
    async fn fetch_one<T: FromRow>(&self, _sql: &str, _values: &[Value]) -> Result<T> {
        unimplemented!("D1 executor is not implemented yet")
    }

    async fn fetch_all<T: FromRow>(&self, _sql: &str, _values: &[Value]) -> Result<Vec<T>> {
        unimplemented!("D1 executor is not implemented yet")
    }

    async fn execute(&self, _sql: &str, _values: &[Value]) -> Result<usize> {
        unimplemented!("D1 executor is not implemented yet")
    }

    fn dialect(&self) -> Dialect {
        Dialect::Sqlite
    }
}
