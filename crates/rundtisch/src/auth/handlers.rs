use crate::auth::config::{
    ACCESS_TTL, AUTH_HASH_PEPPER, AUTH_JWT_ACCESS_SECRET, AUTH_JWT_VERIFY_SECRET, SESSION_COOKIE,
    SESSION_TTL,
};
use crate::auth::error::{AuthError, DbError};
use crate::auth::extract::{AdminUser, BearerUser, secret_bytes};
use crate::auth::jwt::{issue_access_token, issue_activation_token, verify_activation_token};
use crate::auth::models::{NewUser, Role, UpdateUserAlias, User};
use crate::auth::password::{
    Argon2idHasher, PasswordHasher, check_password_policy, dummy_verify,
};
#[cfg(test)]
use crate::auth::password::TestPasswordHasher;
use crate::auth::queries::{
    delete_user as delete_user_row, get_session_by_token_hash, get_user_by_email, get_user_by_id,
    get_user_by_public_id, insert_session, insert_user, list_users as list_user_rows,
    revoke_session, rotate_session, touch_last_login, update_user_alias, verify_email,
};
use crate::auth::session::{
    cap_user_agent, clear_session_cookie_header, cookie_value, generate_session_token,
    session_cookie_header, token_hash,
};
use crate::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use email_address::EmailAddress;
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;
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

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn hasher(state: &AppState) -> Result<Box<dyn PasswordHasher>, AuthError> {
    #[cfg(test)]
    if std::env::var("RUNDTISCH_TEST_PASSWORD_HASHER").ok().as_deref() == Some("1") {
        return Ok(Box::new(TestPasswordHasher));
    }
    let pepper = secret_bytes(state, AUTH_HASH_PEPPER, 32)?;
    Ok(Box::new(Argon2idHasher::new(pepper)))
}

fn normalize_alias(alias: String) -> Result<String, DbError> {
    let trimmed = alias.trim().to_string();
    if trimmed.is_empty() {
        Err(DbError::TypeMismatch)
    } else {
        Ok(trimmed)
    }
}

pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> impl IntoResponse {
    match list_user_rows(&state.db).await {
        Ok(users) => Json(json!({"result": users})).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn create_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(mut new_user): Json<NewUser>,
) -> impl IntoResponse {
    match normalize_alias(new_user.alias) {
        Ok(alias) => new_user.alias = alias,
        Err(e) => return e.into_response(),
    }
    new_user.assign_public_id();
    new_user.stamp_now(now());
    match insert_user(&state.db, &new_user).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({"result": User::from_new(id, new_user)})),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn update_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(public_id): Path<Uuid>,
    Json(body): Json<UpdateUserAlias>,
) -> impl IntoResponse {
    let alias = match normalize_alias(body.alias) {
        Ok(alias) => alias,
        Err(e) => return e.into_response(),
    };
    let updated_at = now();
    match update_user_alias(&state.db, public_id, &alias, updated_at).await {
        Ok(_) => match get_user_by_public_id(&state.db, public_id).await {
            Ok(user) => Json(json!({"result": user})).into_response(),
            Err(e) => e.into_response(),
        },
        Err(e) => e.into_response(),
    }
}

pub async fn delete_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(public_id): Path<Uuid>,
) -> impl IntoResponse {
    match delete_user_row(&state.db, public_id).await {
        Ok(0) => DbError::NotFound.into_response(),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
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

async fn issue_tokens(
    state: &AppState,
    user: &User,
    headers: &HeaderMap,
) -> Result<(String, String), AuthError> {
    let access_secret = secret_bytes(state, AUTH_JWT_ACCESS_SECRET, 32)?;
    let pepper = secret_bytes(state, AUTH_HASH_PEPPER, 32)?;
    let access_token = issue_access_token(user.public_id, user.role, &access_secret, now())?;
    let raw = generate_session_token()
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    let hash = token_hash(&pepper, &raw)
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    let created = now();
    insert_session(
        &state.db,
        user.id,
        &hash,
        created,
        created + SESSION_TTL,
        cap_user_agent(headers).as_deref(),
    )
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

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    match register_inner(state, body).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn register_inner(
    state: AppState,
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
    let password_hasher = hasher(&state)?;
    let password_hash = match password_hasher.hash(&body.password) {
        Ok(hash) => hash,
        Err(err) => {
            body.password.zeroize();
            return Err(err.into());
        }
    };
    body.password.zeroize();
    let mut new_user = NewUser::new(body.email.clone(), alias, Role::User, Some(password_hash));
    new_user.assign_public_id();
    new_user.stamp_now(now());
    let id = insert_user(&state.db, &new_user).await?;
    let user = User::from_new(id, new_user);
    let verify_secret = secret_bytes(&state, AUTH_JWT_VERIFY_SECRET, 32)?;
    let activation_token = issue_activation_token(
        user.public_id,
        user.email.as_ref(),
        &verify_secret,
        now(),
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

pub async fn activate(
    State(state): State<AppState>,
    Json(body): Json<ActivateBody>,
) -> impl IntoResponse {
    match activate_inner(state, body).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn activate_inner(
    state: AppState,
    body: ActivateBody,
) -> Result<axum::response::Response, AuthError> {
    let verify_secret = secret_bytes(&state, AUTH_JWT_VERIFY_SECRET, 32)?;
    let claims = verify_activation_token(&body.token, &verify_secret, now())?;
    let rows = verify_email(&state.db, claims.sub, &claims.email, now()).await?;
    if rows == 0 {
        return Err(AuthError::InvalidToken);
    }
    Ok(Json(json!({"result": "activated"})).into_response())
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    match login_inner(state, headers, body).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn login_inner(
    state: AppState,
    headers: HeaderMap,
    mut body: LoginBody,
) -> Result<axum::response::Response, AuthError> {
    let password_hasher = hasher(&state)?;
    let user = get_user_by_email(&state.db, body.email.as_ref()).await?;
    let Some(user) = user else {
        dummy_verify(password_hasher.as_ref(), &body.password);
        body.password.zeroize();
        return Err(AuthError::InvalidCredentials);
    };
    let Some(password_hash) = user.password_hash.as_deref() else {
        dummy_verify(password_hasher.as_ref(), &body.password);
        body.password.zeroize();
        return Err(AuthError::InvalidCredentials);
    };
    let verified = match password_hasher.verify(&body.password, password_hash) {
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
    let _ = touch_last_login(&state.db, user.public_id, now()).await;
    Ok(login_response(&user, access_token, raw_session))
}

pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match refresh_inner(state, headers).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn refresh_inner(
    state: AppState,
    headers: HeaderMap,
) -> Result<axum::response::Response, AuthError> {
    let raw = cookie_value(&headers, SESSION_COOKIE).ok_or(AuthError::InvalidToken)?;
    let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
    let hash = token_hash(&pepper, &raw)
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    let session = get_session_by_token_hash(&state.db, &hash)
        .await?
        .ok_or(AuthError::InvalidToken)?;
    let created = now();
    if session.revoked_at.is_some() || session.expires_at <= created {
        return Err(AuthError::InvalidToken);
    }
    let user = get_user_by_id(&state.db, session.user_id).await?;
    let new_raw = generate_session_token()
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    let new_hash = token_hash(&pepper, &new_raw)
        .map_err(|err| AuthError::Token(crate::auth::jwt::TokenError::Backend(err)))?;
    rotate_session(&state.db, session.id, &new_hash, created).await?;
    let access_secret = secret_bytes(&state, AUTH_JWT_ACCESS_SECRET, 32)?;
    let access_token = issue_access_token(user.public_id, user.role, &access_secret, now())?;
    Ok(login_response(&user, access_token, new_raw))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match logout_inner(state, headers).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

async fn logout_inner(
    state: AppState,
    headers: HeaderMap,
) -> Result<axum::response::Response, AuthError> {
    if let Some(raw) = cookie_value(&headers, SESSION_COOKIE) {
        let pepper = secret_bytes(&state, AUTH_HASH_PEPPER, 32)?;
        if let Ok(hash) = token_hash(&pepper, &raw) {
            if let Ok(Some(session)) = get_session_by_token_hash(&state.db, &hash).await {
                let _ = revoke_session(&state.db, session.id, now()).await;
            }
        }
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_session_cookie_header());
    Ok(response)
}

pub async fn me(
    State(state): State<AppState>,
    BearerUser(claims): BearerUser,
) -> impl IntoResponse {
    match get_user_by_public_id(&state.db, claims.sub).await {
        Ok(user) => Json(json!({"result": user})).into_response(),
        Err(e) => e.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::migrations;
    use crate::auth::models::{NewUser, Role};
    use crate::auth::queries::{insert_user, verify_email};
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatus};
    use sea_orm_migration::MigratorTrait;
    use tower::ServiceExt;

    const ACCESS_SECRET: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const VERIFY_SECRET: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const PEPPER: &str = "cccccccccccccccccccccccccccccccc";

    struct Migrator;

    impl MigratorTrait for Migrator {
        fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
            migrations::migrations()
        }
    }

    async fn app() -> (axum::Router, sea_orm::DatabaseConnection) {
        unsafe {
            std::env::set_var(AUTH_JWT_ACCESS_SECRET, ACCESS_SECRET);
            std::env::set_var(AUTH_JWT_VERIFY_SECRET, VERIFY_SECRET);
            std::env::set_var(AUTH_HASH_PEPPER, PEPPER);
            std::env::set_var("RUNDTISCH_TEST_PASSWORD_HASHER", "1");
        }
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("sqlite");
        Migrator::up(&db, None).await.expect("migrate");
        let state = AppState { db: db.clone() };
        let router = axum::Router::new()
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
            .with_state(state);
        (router, db)
    }

    async fn seed_verified_user(db: &sea_orm::DatabaseConnection, email: &str, role: Role) {
        let mut new_user = NewUser::new(
            email.parse().unwrap(),
            email.split('@').next().unwrap(),
            role,
            Some("test:unique-passphrase-ok".into()),
        );
        new_user.assign_public_id();
        new_user.stamp_now(OffsetDateTime::now_utc());
        insert_user(db, &new_user).await.expect("insert");
        verify_email(db, new_user.public_id, email, OffsetDateTime::now_utc())
            .await
            .expect("verify");
    }

    async fn login_access_token(app: &axum::Router, email: &str) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","password":"unique-passphrase-ok"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(response).await;
        assert_eq!(status, HttpStatus::OK, "{json}");
        json["access_token"].as_str().unwrap().to_string()
    }

    async fn body_json(response: axum::http::Response<Body>) -> (HttpStatus, serde_json::Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        (status, json)
    }

    fn cookie_header_from_response(response: &axum::http::Response<Body>) -> String {
        response
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .expect("cookie")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    #[tokio::test]
    async fn create_list_delete_users() {
        let (app, db) = app().await;
        seed_verified_user(&db, "admin@example.com", Role::Admin).await;
        let admin = login_access_token(&app, "admin@example.com").await;
        let created = app
            .clone()
            .oneshot(
                Request::post("/api/auth/users")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin}"))
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
        let public_id = json["result"]["public_id"].as_str().unwrap().to_string();

        let listed = app
            .clone()
            .oneshot(
                Request::get("/api/auth/users")
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(listed).await;
        assert_eq!(status, HttpStatus::OK);
        let listed_ids = json["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|user| user["public_id"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert!(listed_ids.contains(&public_id), "{listed_ids:?}");

        let deleted = app
            .oneshot(
                Request::delete(format!("/api/auth/users/{public_id}"))
                    .header("authorization", format!("Bearer {admin}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deleted.status(), HttpStatus::NO_CONTENT);
    }

    #[tokio::test]
    async fn user_management_rejects_missing_and_non_admin_tokens() {
        let (app, db) = app().await;
        let missing = app
            .clone()
            .oneshot(Request::get("/api/auth/users").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(missing.status(), HttpStatus::UNAUTHORIZED);

        seed_verified_user(&db, "carol@example.com", Role::User).await;
        let user = login_access_token(&app, "carol@example.com").await;
        let forbidden = app
            .oneshot(
                Request::get("/api/auth/users")
                    .header("authorization", format!("Bearer {user}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, json) = body_json(forbidden).await;
        assert_eq!(status, HttpStatus::FORBIDDEN);
        assert_eq!(json["error"], "forbidden");
    }

    #[tokio::test]
    async fn register_activate_login_me_refresh_logout() {
        let (app, _) = app().await;
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
        let token = json["activation_token"].as_str().unwrap().to_string();

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
        assert_eq!(activated.status(), HttpStatus::OK);

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
        let (status, json) = body_json(login).await;
        assert_eq!(status, HttpStatus::OK);
        let access = json["access_token"].as_str().unwrap().to_string();

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
        assert_eq!(me.status(), HttpStatus::OK);

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
        assert_eq!(refreshed.status(), HttpStatus::OK);

        let logout = app
            .oneshot(
                Request::post("/api/auth/logout")
                    .header("cookie", cookie_header_from_response(&refreshed))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), HttpStatus::NO_CONTENT);
    }
}
