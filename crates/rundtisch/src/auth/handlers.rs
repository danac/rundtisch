use crate::auth::config::{ACCESS_TTL, SESSION_COOKIE, SESSION_TTL};
use crate::auth::error::AuthError;
use crate::auth::extract::{BearerUser, secret_bytes};
use crate::auth::jwt::{issue_access_token, issue_activation_token, verify_activation_token};
use crate::auth::models::{NewUser, Role, Session, UpdateUserAlias, User};
use crate::auth::password::{check_password_policy, dummy_verify};
use crate::auth::queries::{
    session_get_by_token_hash_query, session_insert_query, session_revoke_query,
    session_rotate_query, user_delete_query, user_get_by_email_query, user_get_by_id_query,
    user_get_query, user_insert_query, user_list_query, user_touch_last_login_query,
    user_update_alias_query, user_verify_email_query,
};
use crate::auth::session::{
    cap_user_agent, clear_session_cookie_header, cookie_value, generate_session_token,
    session_cookie_header, token_hash,
};
use crate::traits::db::{DatabaseExecutor, Error};
use crate::traits::secrets::{HASH_PEPPER, JWT_ACCESS_SECRET, JWT_VERIFY_SECRET};
use crate::{AppState, Platform};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use email_address::EmailAddress;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use zeroize::Zeroize;

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub email: EmailAddress,
    pub password: String,
    pub alias: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateBody {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email: EmailAddress,
    pub password: String,
}

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
    if let Err(e) = new_user.assign_public_id(&*state.random) {
        return AuthError::from(e).into_response();
    }
    new_user.stamp_now(state.clock.now_utc());
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
    Path(public_id): Path<Uuid>,
    Json(body): Json<UpdateUserAlias>,
) -> impl IntoResponse {
    let alias = match normalize_alias(body.alias) {
        Ok(alias) => alias,
        Err(e) => return e.into_response(),
    };
    let updated_at = state.clock.now_utc();
    match state
        .database
        .execute(user_update_alias_query(public_id, &alias, updated_at))
        .await
    {
        Ok(result) if result.rows_affected > 0 => {
            match state
                .database
                .fetch_one::<User>(user_get_query(public_id))
                .await
            {
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
    Path(public_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.database.execute(user_delete_query(public_id)).await {
        Ok(result) if result.rows_affected > 0 => {
            Json(json!({"deleted": public_id})).into_response()
        }
        Ok(_) => Error::NotFound.into_response(),
        Err(e) => e.into_response(),
    }
}

fn user_json(user: &User) -> serde_json::Value {
    json!({
        "public_id": user.public_id,
        "email": user.email,
        "alias": user.alias,
        "role": user.role,
        "email_verified_at": user.email_verified_at.map(crate::auth::models::datetime_to_rfc3339),
    })
}

async fn issue_tokens<P: Platform>(
    state: &AppState<P>,
    user: &User,
    headers: &HeaderMap,
) -> Result<(String, String), AuthError> {
    let access_secret = secret_bytes(&*state.secrets, JWT_ACCESS_SECRET, 32)?;
    let pepper = secret_bytes(&*state.secrets, HASH_PEPPER, 32)?;
    let access_token =
        issue_access_token(user.public_id, user.role, &access_secret, &*state.clock)?;
    let raw = generate_session_token(&*state.random)?;
    let hash = token_hash(&pepper, &raw)
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    let now = state.clock.now_utc();
    state
        .database
        .execute(session_insert_query(
            user.id,
            &hash,
            now,
            now + SESSION_TTL,
            cap_user_agent(headers).as_deref(),
        ))
        .await?;
    Ok((access_token, raw))
}

fn login_response(
    user: &User,
    access_token: String,
    raw_session: String,
) -> axum::response::Response {
    let mut response = (
        StatusCode::OK,
        Json(json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": ACCESS_TTL.whole_seconds(),
            "user": user_json(user),
        })),
    )
        .into_response();
    if let Ok(cookie) = session_cookie_header(&raw_session) {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    response
}

pub async fn register<P: Platform>(
    State(state): State<AppState<P>>,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    match register_inner(state, body).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn register_inner<P: Platform>(
    state: AppState<P>,
    mut body: RegisterBody,
) -> Result<axum::response::Response, AuthError> {
    if let Err(err) = check_password_policy(&body.password) {
        body.password.zeroize();
        return Err(err.into());
    }
    let alias = match body.alias {
        Some(alias) => normalize_alias(alias).map_err(|_| AuthError::TypeMismatch)?,
        None => body
            .email
            .as_ref()
            .split('@')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("user")
            .to_string(),
    };
    let password_hash = match state.password_hasher.hash(&body.password) {
        Ok(hash) => hash,
        Err(err) => {
            body.password.zeroize();
            return Err(err.into());
        }
    };
    body.password.zeroize();
    let mut new_user = NewUser::new(body.email.clone(), alias, Role::User, Some(password_hash));
    new_user.assign_public_id(&*state.random)?;
    new_user.stamp_now(state.clock.now_utc());
    let id = state.database.insert(user_insert_query(&new_user)).await?;
    let user = User::from_new(id, new_user);
    let verify_secret = secret_bytes(&*state.secrets, JWT_VERIFY_SECRET, 32)?;
    let activation_token = issue_activation_token(
        user.public_id,
        user.email.as_ref(),
        &verify_secret,
        &*state.clock,
    )?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "result": user_json(&user),
            "activation_token": activation_token,
        })),
    )
        .into_response())
}

pub async fn activate<P: Platform>(
    State(state): State<AppState<P>>,
    Json(body): Json<ActivateBody>,
) -> impl IntoResponse {
    match activate_inner(state, body).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn activate_inner<P: Platform>(
    state: AppState<P>,
    body: ActivateBody,
) -> Result<axum::response::Response, AuthError> {
    let verify_secret = secret_bytes(&*state.secrets, JWT_VERIFY_SECRET, 32)?;
    let claims = verify_activation_token(&body.token, &verify_secret, &*state.clock)?;
    let result = state
        .database
        .execute(user_verify_email_query(
            claims.sub,
            &claims.email,
            state.clock.now_utc(),
        ))
        .await?;
    if result.rows_affected == 0 {
        return Err(AuthError::InvalidToken);
    }
    Ok(Json(json!({"result": "activated"})).into_response())
}

pub async fn login<P: Platform>(
    State(state): State<AppState<P>>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    match login_inner(state, headers, body).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn login_inner<P: Platform>(
    state: AppState<P>,
    headers: HeaderMap,
    mut body: LoginBody,
) -> Result<axum::response::Response, AuthError> {
    let user = state
        .database
        .fetch_optional::<User>(user_get_by_email_query(body.email.as_ref()))
        .await?;
    let Some(user) = user else {
        dummy_verify(&*state.password_hasher, &body.password);
        body.password.zeroize();
        return Err(AuthError::InvalidCredentials);
    };
    let Some(password_hash) = user.password_hash.as_deref() else {
        dummy_verify(&*state.password_hasher, &body.password);
        body.password.zeroize();
        return Err(AuthError::InvalidCredentials);
    };
    let verified = match state.password_hasher.verify(&body.password, password_hash) {
        Ok(ok) => ok,
        Err(err) => {
            body.password.zeroize();
            return Err(err.into());
        }
    };
    body.password.zeroize();
    if !verified {
        return Err(AuthError::InvalidCredentials);
    }
    if user.email_verified_at.is_none() {
        return Err(AuthError::EmailNotVerified);
    }
    let (access_token, raw_session) = issue_tokens(&state, &user, &headers).await?;
    let _ = state
        .database
        .execute(user_touch_last_login_query(
            user.public_id,
            state.clock.now_utc(),
        ))
        .await;
    Ok(login_response(&user, access_token, raw_session))
}

pub async fn refresh<P: Platform>(
    State(state): State<AppState<P>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match refresh_inner(state, headers).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn refresh_inner<P: Platform>(
    state: AppState<P>,
    headers: HeaderMap,
) -> Result<axum::response::Response, AuthError> {
    let raw = cookie_value(&headers, SESSION_COOKIE).ok_or(AuthError::InvalidToken)?;
    let pepper = secret_bytes(&*state.secrets, HASH_PEPPER, 32)?;
    let hash = token_hash(&pepper, &raw)
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    let session = state
        .database
        .fetch_optional::<Session>(session_get_by_token_hash_query(&hash))
        .await?
        .ok_or(AuthError::InvalidToken)?;
    let now = state.clock.now_utc();
    if session.revoked_at.is_some() || session.expires_at <= now {
        return Err(AuthError::InvalidToken);
    }
    let user = state
        .database
        .fetch_one::<User>(user_get_by_id_query(session.user_id))
        .await?;
    let new_raw = generate_session_token(&*state.random)?;
    let new_hash = token_hash(&pepper, &new_raw)
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    state
        .database
        .execute(session_rotate_query(session.id, &new_hash, now))
        .await?;
    let access_secret = secret_bytes(&*state.secrets, JWT_ACCESS_SECRET, 32)?;
    let access_token =
        issue_access_token(user.public_id, user.role, &access_secret, &*state.clock)?;
    Ok(login_response(&user, access_token, new_raw))
}

pub async fn logout<P: Platform>(
    State(state): State<AppState<P>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match logout_inner(state, headers).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn logout_inner<P: Platform>(
    state: AppState<P>,
    headers: HeaderMap,
) -> Result<axum::response::Response, AuthError> {
    if let Some(raw) = cookie_value(&headers, SESSION_COOKIE) {
        let pepper = secret_bytes(&*state.secrets, HASH_PEPPER, 32)?;
        if let Ok(hash) = token_hash(&pepper, &raw) {
            if let Ok(Some(session)) = state
                .database
                .fetch_optional::<Session>(session_get_by_token_hash_query(&hash))
                .await
            {
                let _ = state
                    .database
                    .execute(session_revoke_query(session.id, state.clock.now_utc()))
                    .await;
            }
        }
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_session_cookie_header());
    Ok(response)
}

pub async fn me<P: Platform>(
    State(state): State<AppState<P>>,
    BearerUser(claims): BearerUser,
) -> impl IntoResponse {
    match state
        .database
        .fetch_one::<User>(user_get_query(claims.sub))
        .await
    {
        Ok(user) => Json(json!({"result": user})).into_response(),
        Err(e) => e.into_response(),
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::adapters::clock::SystemClock;
    use crate::adapters::db::sqlite::SqliteExecutor;
    use crate::adapters::secrets::MapSecretStore;
    use crate::auth::migrations::auth_migration_001::AuthMigration001;
    use crate::auth::models::Role;
    use crate::auth::password::TestPasswordHasher;
    use crate::traits::clock::Clock;
    use crate::traits::db::{Dialect, Migration, schema_to_sql};
    use crate::traits::random::RandomSource;
    use crate::traits::secrets::{HASH_PEPPER, JWT_ACCESS_SECRET, JWT_VERIFY_SECRET};
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    const ACCESS_SECRET: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const VERIFY_SECRET: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const PEPPER: &str = "cccccccccccccccccccccccccccccccc";

    struct TestPlatform {
        db: Arc<SqliteExecutor>,
        secrets: Arc<MapSecretStore>,
        random: Arc<dyn RandomSource>,
        clock: Arc<dyn Clock>,
    }

    impl Platform for TestPlatform {
        type Database = SqliteExecutor;
        type SecretStore = MapSecretStore;

        fn database(&self) -> Arc<Self::Database> {
            self.db.clone()
        }

        fn secrets(&self) -> Arc<Self::SecretStore> {
            self.secrets.clone()
        }

        fn random(&self) -> Arc<dyn RandomSource> {
            self.random.clone()
        }

        fn clock(&self) -> Arc<dyn Clock> {
            self.clock.clone()
        }
    }

    fn test_secrets() -> MapSecretStore {
        MapSecretStore::new([
            (JWT_ACCESS_SECRET, ACCESS_SECRET),
            (JWT_VERIFY_SECRET, VERIFY_SECRET),
            (HASH_PEPPER, PEPPER),
        ])
    }

    async fn migrated_pool() -> sqlx::SqlitePool {
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
        pool
    }

    async fn app() -> axum::Router {
        let pool = migrated_pool().await;
        let platform = TestPlatform {
            db: Arc::new(SqliteExecutor::from_pool(pool)),
            secrets: Arc::new(test_secrets()),
            random: Arc::new(crate::adapters::random::OsRandom),
            clock: Arc::new(SystemClock),
        };
        let mut state = AppState::<TestPlatform>::from_platform(&platform);
        state.password_hasher = Arc::new(TestPasswordHasher);
        axum::Router::new()
            .route(
                "/api/auth/users",
                axum::routing::get(list_users).post(create_user),
            )
            .route(
                "/api/auth/users/{public_id}",
                axum::routing::patch(update_user).delete(delete_user),
            )
            .route("/api/auth/register", axum::routing::post(register))
            .route("/api/auth/activate", axum::routing::post(activate))
            .route("/api/auth/login", axum::routing::post(login))
            .route("/api/auth/refresh", axum::routing::post(refresh))
            .route("/api/auth/logout", axum::routing::post(logout))
            .route("/api/auth/me", axum::routing::get(me))
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
        assert!(json["result"]["id"].is_null() || json["result"].get("id").is_none());
        let public_id = json["result"]["public_id"].as_str().unwrap().to_string();
        uuid::Uuid::parse_str(&public_id).expect("public_id is a uuid");

        let listed = app
            .clone()
            .oneshot(Request::get("/api/auth/users").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (status, json) = body_json(listed).await;
        assert_eq!(status, HttpStatus::OK);
        let listed_users = json["result"].as_array().unwrap();
        assert_eq!(listed_users.len(), 1);
        assert_eq!(listed_users[0]["public_id"], public_id);
        assert!(listed_users[0].get("id").is_none());

        let deleted = app
            .clone()
            .oneshot(
                Request::delete(format!("/api/auth/users/{public_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(deleted).await;
        assert_eq!(status, HttpStatus::OK);
        assert_eq!(json["deleted"], public_id);

        let missing = app
            .oneshot(
                Request::delete(format!("/api/auth/users/{public_id}"))
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
    async fn create_ignores_client_public_id() {
        let app = app().await;
        let created = app
            .oneshot(
                Request::post("/api/auth/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"eve@example.com","alias":"eve","role":"User","public_id":"11111111-1111-4111-8111-111111111111"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(created).await;
        assert_eq!(status, HttpStatus::CREATED);
        let public_id = json["result"]["public_id"].as_str().unwrap();
        assert_ne!(public_id, "11111111-1111-4111-8111-111111111111");
        uuid::Uuid::parse_str(public_id).expect("public_id is a uuid");
        assert!(json["result"].get("id").is_none());
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
        let public_id = json["result"]["public_id"].as_str().unwrap().to_string();

        let updated = app
            .clone()
            .oneshot(
                Request::patch(format!("/api/auth/users/{public_id}"))
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
                Request::patch(format!("/api/auth/users/{public_id}"))
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
                Request::patch("/api/auth/users/ffffffff-ffff-4fff-8fff-ffffffffffff")
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

    fn cookie_header_from_response(response: &axum::http::Response<Body>) -> String {
        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .unwrap();
        set_cookie
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    #[tokio::test]
    async fn register_activate_login_me_refresh_logout() {
        let app = app().await;
        let registered = app
            .clone()
            .oneshot(
                Request::post("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"carol@example.com","password":"unique-passphrase-ok","alias":"carol"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(registered).await;
        assert_eq!(status, HttpStatus::CREATED);
        assert_eq!(json["result"]["email"], "carol@example.com");
        assert!(json["result"].get("password_hash").is_none());
        let token = json["activation_token"].as_str().unwrap().to_string();

        let too_early = app
            .clone()
            .oneshot(
                Request::post("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"carol@example.com","password":"unique-passphrase-ok"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(too_early).await;
        assert_eq!(status, HttpStatus::FORBIDDEN);
        assert_eq!(json["error"], "email_not_verified");

        let activated = app
            .clone()
            .oneshot(
                Request::post("/api/auth/activate")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"token":"{token}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(activated).await;
        assert_eq!(status, HttpStatus::OK);
        assert_eq!(json["result"], "activated");

        let login = app
            .clone()
            .oneshot(
                Request::post("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"carol@example.com","password":"unique-passphrase-ok"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let cookie = cookie_header_from_response(&login);
        assert!(cookie.starts_with("session="));
        let (status, json) = body_json(login).await;
        assert_eq!(status, HttpStatus::OK);
        let access = json["access_token"].as_str().unwrap().to_string();
        assert_eq!(json["user"]["alias"], "carol");

        let me = app
            .clone()
            .oneshot(
                Request::get("/api/auth/me")
                    .header("authorization", format!("Bearer {access}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(me).await;
        assert_eq!(status, HttpStatus::OK);
        assert_eq!(json["result"]["email"], "carol@example.com");
        assert!(json["result"].get("password_hash").is_none());

        let unknown = app
            .clone()
            .oneshot(
                Request::post("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"nobody@example.com","password":"unique-passphrase-ok"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(unknown).await;
        assert_eq!(status, HttpStatus::UNAUTHORIZED);
        assert_eq!(json["error"], "invalid_credentials");

        let refreshed = app
            .clone()
            .oneshot(
                Request::post("/api/auth/refresh")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let new_cookie = cookie_header_from_response(&refreshed);
        assert_ne!(new_cookie, cookie);
        let (status, json) = body_json(refreshed).await;
        assert_eq!(status, HttpStatus::OK);
        assert!(json["access_token"].as_str().is_some());

        let reused = app
            .clone()
            .oneshot(
                Request::post("/api/auth/refresh")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(reused).await;
        assert_eq!(status, HttpStatus::UNAUTHORIZED);
        assert_eq!(json["error"], "invalid_token");

        let logout = app
            .clone()
            .oneshot(
                Request::post("/api/auth/logout")
                    .header("cookie", &new_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), HttpStatus::NO_CONTENT);

        let after_logout = app
            .oneshot(
                Request::post("/api/auth/refresh")
                    .header("cookie", &new_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(after_logout).await;
        assert_eq!(status, HttpStatus::UNAUTHORIZED);
        assert_eq!(json["error"], "invalid_token");
    }

    #[tokio::test]
    async fn register_rejects_short_password() {
        let app = app().await;
        let registered = app
            .oneshot(
                Request::post("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"carol@example.com","password":"short"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(registered).await;
        assert_eq!(status, HttpStatus::BAD_REQUEST);
        assert_eq!(json["error"], "invalid_password");
    }
}
