use crate::handlers::health;
use crate::library::{get_collection, list_collections, list_photos, photo_file};
use axum::Router;
use axum::routing::{delete, get, post, put};
use rundtisch::AppState;
use rundtisch::auth::handlers::{
    clear_password, list_passkeys, list_sessions, login, logout, logout_all, me, mint_session,
    passkey_delete, passkey_login, passkey_login_options, passkey_register,
    passkey_register_options, preview_invitation, register_passkey, register_passkey_options,
    register_password, register_with_token, request_reset, reset_passkey, reset_passkey_options,
    reset_password, revoke_listed_session, set_alias, set_password, step_up_login,
    step_up_passkey_login, step_up_passkey_login_options,
};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/collections", get(list_collections))
        .route("/api/collections/{id}", get(get_collection))
        .route("/api/collections/{id}/photos", get(list_photos))
        .route("/api/photos/{id}/file", get(photo_file))
        .route("/api/auth/register_with_token", post(register_with_token))
        .route("/api/auth/register/invitation", post(preview_invitation))
        .route("/api/auth/register/password", post(register_password))
        .route(
            "/api/auth/register/passkey/options",
            post(register_passkey_options),
        )
        .route("/api/auth/register/passkey", post(register_passkey))
        .route("/api/auth/login", post(login))
        .route(
            "/api/auth/passkeys/login/options",
            post(passkey_login_options),
        )
        .route("/api/auth/passkeys/login", post(passkey_login))
        .route("/api/auth/step-up/login", post(step_up_login))
        .route(
            "/api/auth/step-up/passkeys/login/options",
            post(step_up_passkey_login_options),
        )
        .route(
            "/api/auth/step-up/passkeys/login",
            post(step_up_passkey_login),
        )
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/logout_all", post(logout_all))
        .route("/api/auth/me", get(me))
        .route("/api/auth/alias", put(set_alias))
        .route("/api/auth/sessions", get(list_sessions).post(mint_session))
        .route(
            "/api/auth/sessions/{public_id}",
            delete(revoke_listed_session),
        )
        .route(
            "/api/auth/password",
            put(set_password).delete(clear_password),
        )
        .route("/api/auth/request_reset", post(request_reset))
        .route("/api/auth/reset", post(reset_password))
        .route(
            "/api/auth/reset/passkey/options",
            post(reset_passkey_options),
        )
        .route("/api/auth/reset/passkey", post(reset_passkey))
        .route("/api/auth/passkeys", get(list_passkeys))
        .route(
            "/api/auth/passkeys/register/options",
            post(passkey_register_options),
        )
        .route("/api/auth/passkeys/register", post(passkey_register))
        .route("/api/auth/passkeys/{public_id}", delete(passkey_delete))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use rundtisch::auth::models::{NewUser, Role};
    use rundtisch::auth::queries::{insert_session, insert_user};
    use rundtisch::auth::session::token_hash;
    use rundtisch::{AppState, EmailSendError, EmailSender};
    use sea_orm::ConnectOptions;
    use time::{Duration, OffsetDateTime};
    use tower::ServiceExt;

    use super::build_router;
    use crate::library::catalog::{SERENITY_SPA_COVER_ID, SERENITY_SPA_ID, mock_catalog};
    use crate::library::seed::{BytesSource, seed_catalog};
    use crate::library::store::PNG_1X1;
    use crate::migrate;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct SilentMail;

    #[async_trait::async_trait]
    impl EmailSender for SilentMail {
        async fn send(&self, _to: &str, _subject: &str, _body: &str) -> Result<(), EmailSendError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn library_routes_require_a_session_and_serve_seeded_files() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "crockis-http-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pepper = "cccccccccccccccccccccccccccccccc";
        unsafe {
            std::env::set_var("AUTH_HASH_PEPPER", pepper);
            std::env::set_var("DATA_DIR", &dir);
        }

        let mut opts = ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1);
        let db = sea_orm::Database::connect(opts).await.unwrap();
        migrate(&db).await.unwrap();
        let source = BytesSource {
            bytes: PNG_1X1.to_vec(),
            hits: std::sync::atomic::AtomicUsize::new(0),
        };
        seed_catalog(&db, &dir, mock_catalog(), &source)
            .await
            .unwrap();

        let now = OffsetDateTime::now_utc();
        let user_id = insert_user(
            &db,
            &NewUser::new(
                "library@example.com".parse().unwrap(),
                "Library",
                Role::User,
                None,
            ),
        )
        .await
        .unwrap();
        let raw = "library-session-token";
        insert_session(
            &db,
            user_id,
            &token_hash(pepper.as_bytes(), raw).unwrap(),
            now,
            now + Duration::hours(2),
            None,
        )
        .await
        .unwrap();

        let state = AppState {
            db,
            email: Arc::new(SilentMail),
        };
        let app = build_router(state);
        let cookie = format!("session={raw}");

        let anonymous = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/collections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

        let listed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/collections")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(listed.status(), StatusCode::OK);
        let body = to_bytes(listed.into_body(), usize::MAX).await.unwrap();
        let collections: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let collections = collections.as_array().unwrap();
        assert_eq!(collections.len(), 4);
        assert_eq!(collections[0]["id"], SERENITY_SPA_ID.to_string());
        assert_eq!(collections[0]["name"], "Serenity Spa");
        assert_eq!(collections[0]["photoCount"], 16);
        assert_eq!(
            collections[0]["cover"]["id"],
            SERENITY_SPA_COVER_ID.to_string()
        );
        assert_eq!(collections[0]["cover"]["width"], 1);
        assert_eq!(collections[0]["cover"]["filename"], "Heat.png");
        assert!(collections[0]["cover"].get("alt").is_none());
        assert!(collections[0]["cover"].get("title").is_none());
        let src = collections[0]["cover"]["src"].as_str().unwrap().to_owned();

        let malformed = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/collections/missing")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/collections/018f5c10-0000-7000-8000-0000000000ff")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);

        let photos = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/collections/{SERENITY_SPA_ID}/photos"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(photos.status(), StatusCode::OK);
        let photos: serde_json::Value =
            serde_json::from_slice(&to_bytes(photos.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(photos.as_array().unwrap().len(), 16);
        assert_eq!(photos[0]["collectionId"], SERENITY_SPA_ID.to_string());
        assert_eq!(photos[0]["filename"], "Stones.png");
        assert!(photos[0].get("alt").is_none());
        assert!(photos[0].get("title").is_none());
        assert!(
            photos[0]["takenAt"]
                .as_str()
                .unwrap()
                .starts_with("2026-03-12T10:00:00")
        );

        let file = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&src)
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(file.status(), StatusCode::OK);
        assert_eq!(file.headers().get("content-type").unwrap(), "image/png");
        let bytes = to_bytes(file.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&bytes[..8], &PNG_1X1[..8]);

        let hidden = app
            .oneshot(Request::builder().uri(&src).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(hidden.status(), StatusCode::UNAUTHORIZED);

        std::fs::remove_dir_all(&dir).ok();
    }
}
