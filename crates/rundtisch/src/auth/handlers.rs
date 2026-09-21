use crate::auth::models::{NewUser, UpdateUserAlias, User};
use crate::auth::queries::{
    user_delete_query, user_get_query, user_insert_query, user_list_query, user_update_alias_query,
};
use crate::traits::db::{DatabaseExecutor, Error};
use crate::{AppState, Platform};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Conflict => StatusCode::CONFLICT,
            Error::TypeMismatch => StatusCode::BAD_REQUEST,
            Error::Backend(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({"error": self.to_string()}))).into_response()
    }
}

fn normalize_alias(alias: String) -> Result<String, Error> {
    let trimmed = alias.trim().to_string();
    if trimmed.is_empty() {
        Err(Error::TypeMismatch)
    } else {
        Ok(trimmed)
    }
}

pub async fn list_users<P: Platform>(State(state): State<AppState<P>>) -> impl IntoResponse {
    let query = user_list_query();
    let result = state.database.fetch_all::<User>(query).await;
    match result {
        Ok(users) => Json(json!({"result": users})).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn create_user<P: Platform>(
    State(state): State<AppState<P>>,
    Json(mut new_user): Json<NewUser>,
) -> impl IntoResponse {
    match normalize_alias(new_user.alias) {
        Ok(alias) => new_user.alias = alias,
        Err(e) => return e.into_response(),
    }
    new_user.stamp_now();
    match state.database.insert(user_insert_query(&new_user)).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({"result": User::from_new(id, new_user)})),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn update_user<P: Platform>(
    State(state): State<AppState<P>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUserAlias>,
) -> impl IntoResponse {
    let alias = match normalize_alias(body.alias) {
        Ok(alias) => alias,
        Err(e) => return e.into_response(),
    };
    let updated_at = time::OffsetDateTime::now_utc();
    match state
        .database
        .execute(user_update_alias_query(id, &alias, updated_at))
        .await
    {
        Ok(result) if result.rows_affected > 0 => {
            match state.database.fetch_one::<User>(user_get_query(id)).await {
                Ok(user) => Json(json!({"result": user})).into_response(),
                Err(e) => e.into_response(),
            }
        }
        Ok(_) => Error::NotFound.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn delete_user<P: Platform>(
    State(state): State<AppState<P>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.database.execute(user_delete_query(id)).await {
        Ok(result) if result.rows_affected > 0 => Json(json!({"deleted": id})).into_response(),
        Ok(_) => Error::NotFound.into_response(),
        Err(e) => e.into_response(),
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::adapters::db::sqlite::SqliteExecutor;
    use crate::auth::migrations::auth_migration_001::AuthMigration001;
    use crate::auth::models::Role;
    use crate::traits::db::{schema_to_sql, Dialect, Migration};
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    struct TestPlatform {
        db: Arc<SqliteExecutor>,
    }

    impl Platform for TestPlatform {
        type Database = SqliteExecutor;

        fn database(&self) -> Arc<Self::Database> {
            self.db.clone()
        }
    }

    async fn app() -> axum::Router {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect");
        for stmt in AuthMigration001.up() {
            let sql = schema_to_sql(&stmt, Dialect::Sqlite);
            sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .execute(&pool)
                .await
                .expect("migrate");
        }
        let state = AppState::<TestPlatform>::from_platform(&TestPlatform {
            db: Arc::new(SqliteExecutor::from_pool(pool)),
        });
        axum::Router::new()
            .route("/api/auth/users", axum::routing::get(list_users).post(create_user))
            .route(
                "/api/auth/users/{id}",
                axum::routing::patch(update_user).delete(delete_user),
            )
            .with_state(state)
    }

    async fn body_json(response: axum::http::Response<Body>) -> (HttpStatus, serde_json::Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        (status, json)
    }

    #[tokio::test]
    async fn create_list_delete_users() {
        let app = app().await;

        let created = app
            .clone()
            .oneshot(
                Request::post("/api/auth/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"carol@example.com","alias":"carol","role":"User"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(created).await;
        assert_eq!(status, HttpStatus::CREATED);
        assert_eq!(json["result"]["email"], "carol@example.com");
        assert_eq!(json["result"]["alias"], "carol");
        assert_eq!(json["result"]["role"], "User");
        let id = json["result"]["id"].as_i64().unwrap();

        let listed = app
            .clone()
            .oneshot(Request::get("/api/auth/users").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (status, json) = body_json(listed).await;
        assert_eq!(status, HttpStatus::OK);
        assert_eq!(json["result"].as_array().unwrap().len(), 1);

        let deleted = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/auth/users/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(deleted).await;
        assert_eq!(status, HttpStatus::OK);
        assert_eq!(json["deleted"], id);

        let missing = app
            .oneshot(
                Request::delete(format!("/api/auth/users/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(missing).await;
        assert_eq!(status, HttpStatus::NOT_FOUND);
        assert_eq!(json["error"], "not found");
    }

    #[tokio::test]
    async fn create_trims_alias_whitespace() {
        let app = app().await;
        let created = app
            .oneshot(
                Request::post("/api/auth/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"carol@example.com","alias":"  carol  ","role":"User"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(created).await;
        assert_eq!(status, HttpStatus::CREATED);
        assert_eq!(json["result"]["alias"], "carol");
    }

    #[tokio::test]
    async fn update_user_alias() {
        let app = app().await;
        let created = app
            .clone()
            .oneshot(
                Request::post("/api/auth/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"carol@example.com","alias":"carol","role":"User"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(created).await;
        assert_eq!(status, HttpStatus::CREATED);
        let id = json["result"]["id"].as_i64().unwrap();

        let updated = app
            .clone()
            .oneshot(
                Request::patch(format!("/api/auth/users/{id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"alias":"  carolyn  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(updated).await;
        assert_eq!(status, HttpStatus::OK);
        assert_eq!(json["result"]["alias"], "carolyn");
        assert_eq!(json["result"]["email"], "carol@example.com");

        let blank = app
            .clone()
            .oneshot(
                Request::patch(format!("/api/auth/users/{id}"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"alias":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(blank).await;
        assert_eq!(status, HttpStatus::BAD_REQUEST);
        assert_eq!(json["error"], "type mismatch");

        let missing = app
            .oneshot(
                Request::patch("/api/auth/users/999")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"alias":"ghost"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(missing).await;
        assert_eq!(status, HttpStatus::NOT_FOUND);
        assert_eq!(json["error"], "not found");
    }

    #[tokio::test]
    async fn create_duplicate_email_conflicts() {
        let app = app().await;
        let body = r#"{"email":"dave@example.com","alias":"dave","role":"Admin"}"#;
        let first = app
            .clone()
            .oneshot(
                Request::post("/api/auth/users")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), HttpStatus::CREATED);

        let second = app
            .oneshot(
                Request::post("/api/auth/users")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(second).await;
        assert_eq!(status, HttpStatus::CONFLICT);
        assert_eq!(json["error"], "conflict");
    }

    #[test]
    fn role_as_str() {
        assert_eq!(Role::User.as_str(), "User");
        assert_eq!(Role::Admin.as_str(), "Admin");
    }

    #[test]
    fn normalize_alias_trims_and_rejects_blank() {
        assert_eq!(normalize_alias("  a  ".into()).unwrap(), "a");
        assert!(matches!(
            normalize_alias("   ".into()),
            Err(Error::TypeMismatch)
        ));
    }
}
