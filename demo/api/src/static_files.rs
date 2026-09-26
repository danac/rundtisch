use std::path::{Path, PathBuf};

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

/// Guest path mapped in `demo/wasmer.toml` (`[fs] "/app/web" = "./web/dist"`).
pub const DEFAULT_STATIC_DIR: &str = "/app/web";

/// Directory to serve the built SPA from, if it exists on this host.
///
/// `STATIC_DIR` overrides the Wasmer mount. Split-dev (`npm run dev --prefix demo`)
/// leaves this unset so Vite keeps serving the frontend.
pub fn static_dir() -> Option<PathBuf> {
    resolve_static_dir(std::env::var("STATIC_DIR").ok().as_deref())
}

pub fn resolve_static_dir(override_dir: Option<&str>) -> Option<PathBuf> {
    let dir = override_dir
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_STATIC_DIR);
    let path = PathBuf::from(dir);
    path.is_dir().then_some(path)
}

pub fn frontend_service(dir: &Path) -> ServeDir<ServeFile> {
    ServeDir::new(dir)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(dir.join("index.html")))
}

pub fn with_frontend<S>(router: Router<S>, dir: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.fallback_service(frontend_service(dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    fn unique_temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rundtisch-static-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp static dir");
        dir
    }

    #[test]
    fn missing_default_mount_is_skipped() {
        assert_eq!(
            resolve_static_dir(None).is_some(),
            Path::new(DEFAULT_STATIC_DIR).is_dir()
        );
        assert_eq!(
            resolve_static_dir(Some("/definitely-not-a-rundtisch-static-dir")),
            None
        );
    }

    #[test]
    fn existing_override_is_used() {
        let dir = unique_temp_dir();
        assert_eq!(
            resolve_static_dir(Some(dir.to_str().expect("utf8"))),
            Some(dir.clone())
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn blank_override_falls_back_to_default_mount() {
        assert_eq!(
            resolve_static_dir(Some("  ")).is_some(),
            Path::new(DEFAULT_STATIC_DIR).is_dir()
        );
    }

    #[tokio::test]
    async fn serves_index_and_assets_from_dir() {
        let dir = unique_temp_dir();
        std::fs::write(dir.join("index.html"), "<html>spa</html>").unwrap();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        std::fs::write(dir.join("assets/app.js"), "console.log(1)").unwrap();

        let app = with_frontend(Router::new(), &dir);

        let index = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(index.status(), StatusCode::OK);
        let body = to_bytes(index.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"<html>spa</html>");

        let asset = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(asset.status(), StatusCode::OK);

        let spa_fallback = app
            .oneshot(
                Request::builder()
                    .uri("/login")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(spa_fallback.status(), StatusCode::OK);
        let fallback_body = to_bytes(spa_fallback.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&fallback_body[..], b"<html>spa</html>");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn built_frontend_dist_is_servable_when_present() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web/dist");
        if !dir.join("index.html").is_file() {
            return;
        }
        let app = with_frontend(Router::new(), &dir);
        let res = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(
            html.contains("rundtisch") || html.contains("root"),
            "expected the Vite index shell, got {html}"
        );
    }

    #[tokio::test]
    async fn api_routes_win_over_static_fallback() {
        let dir = unique_temp_dir();
        std::fs::write(dir.join("index.html"), "<html>spa</html>").unwrap();
        let app = with_frontend(
            Router::new().route("/api/health", get(|| async { "ok" })),
            &dir,
        );
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"ok");
        std::fs::remove_dir_all(&dir).ok();
    }
}
