use axum::body::Body;
use axum::http::header::{
    self, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, ETAG,
    IF_MODIFIED_SINCE, IF_NONE_MATCH, IF_RANGE, LOCATION, RANGE, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, HeaderValue, Method, Response, StatusCode};
use axum::response::IntoResponse;
use bytes::Bytes;
use time::OffsetDateTime;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::delivery::{ContentDelivery, Delivery, DeliveryMeta, LocalFile, RedirectDelivery};

impl Delivery {
    /// Turn a delivery into an HTTP response.
    ///
    /// `If-None-Match` is checked against the ETag first, so a match is a 304
    /// without opening the file. `File` then lets `ServeFile` handle range
    /// requests, `If-Modified-Since`, and `HEAD`.
    pub async fn respond(self, method: &Method, headers: &HeaderMap) -> Response<Body> {
        if let Some(etag) = self.etag() {
            if if_none_match(headers, &etag) {
                return not_modified(&etag, &self.cache_header());
            }
        }
        match self {
            Delivery::File(file) => serve_file(file, method, headers).await,
            Delivery::Content(content) => serve_content(content, method, headers),
            Delivery::Redirect(redirect) => serve_redirect(redirect),
        }
    }

    fn etag(&self) -> Option<String> {
        match self {
            Delivery::File(file) => file.meta.etag.clone(),
            Delivery::Content(content) => content.meta.etag.clone(),
            Delivery::Redirect(_) => None,
        }
    }

    fn cache_header(&self) -> String {
        match self {
            Delivery::File(file) => file.meta.cache.header_value(),
            Delivery::Content(content) => content.meta.cache.header_value(),
            Delivery::Redirect(_) => "private, max-age=0".to_owned(),
        }
    }
}

fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    let Some(value) = headers.get(IF_NONE_MATCH) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    let ours = etag.trim_matches('"');
    for part in value.split(',') {
        let part = part.trim();
        if part == "*" {
            return true;
        }
        let part = part.strip_prefix("W/").unwrap_or(part).trim();
        if part == etag || part.trim_matches('"') == ours {
            return true;
        }
    }
    false
}

fn not_modified(etag: &str, cache: &str) -> Response<Body> {
    let mut response = Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .body(Body::empty())
        .expect("304 response");
    decorate(response.headers_mut(), None, Some(etag), None, Some(cache));
    response
}

fn decorate(
    headers: &mut HeaderMap,
    content_type: Option<&str>,
    etag: Option<&str>,
    disposition: Option<&str>,
    cache: Option<&str>,
) {
    if let Some(content_type) = content_type {
        if !headers.contains_key(CONTENT_TYPE) {
            insert(headers, CONTENT_TYPE, content_type);
        }
    }
    if let Some(etag) = etag {
        insert(headers, ETAG, etag);
    }
    if let Some(disposition) = disposition {
        insert(headers, CONTENT_DISPOSITION, disposition);
    }
    if let Some(cache) = cache {
        insert(headers, CACHE_CONTROL, cache);
    }
    insert(headers, X_CONTENT_TYPE_OPTIONS, "nosniff");
}

fn insert(headers: &mut HeaderMap, name: header::HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        headers.insert(name, value);
    }
}

fn meta_parts(meta: &DeliveryMeta) -> (Option<String>, Option<String>) {
    let disposition = meta.disposition.as_ref().map(|value| value.header_value());
    let cache = meta.cache.header_value();
    (disposition, Some(cache))
}

async fn serve_file(file: LocalFile, method: &Method, headers: &HeaderMap) -> Response<Body> {
    let mime = file
        .meta
        .content_type
        .parse::<mime::Mime>()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);
    let service = ServeFile::new_with_mime(&file.path, &mime);
    let mut builder = axum::http::Request::builder()
        .method(method.clone())
        .uri("/");
    for name in [RANGE, IF_MODIFIED_SINCE, IF_RANGE] {
        if let Some(value) = headers.get(&name) {
            builder = builder.header(name, value);
        }
    }
    let request = builder.body(Body::empty()).expect("serve file request");
    let response = match service.oneshot(request).await {
        Ok(response) => response,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("unable to read stored file: {err}"),
            )
                .into_response();
        }
    };
    let (mut parts, body) = response.into_parts();
    let (disposition, cache) = meta_parts(&file.meta);
    decorate(
        &mut parts.headers,
        Some(&file.meta.content_type),
        file.meta.etag.as_deref(),
        disposition.as_deref(),
        cache.as_deref(),
    );
    Response::from_parts(parts, Body::new(body))
}

fn serve_content(content: ContentDelivery, method: &Method, headers: &HeaderMap) -> Response<Body> {
    if let Some(modified) = content.meta.last_modified {
        if not_modified_since(headers, modified) {
            return not_modified(
                content.meta.etag.as_deref().unwrap_or("\"\""),
                &content.meta.cache.header_value(),
            );
        }
    }
    let total = content.bytes.len() as u64;
    match slice_header(&content.bytes, headers.get(RANGE)) {
        RangeSlice::Ignore => {
            let data = content.bytes.clone();
            respond_bytes(content, method, data, None)
        }
        RangeSlice::NotSatisfiable => range_not_satisfiable(total, &content.meta),
        RangeSlice::Bytes { start, data } => {
            let end = start + data.len() as u64 - 1;
            let content_range = format!("bytes {start}-{end}/{total}");
            respond_bytes(content, method, data, Some(content_range))
        }
    }
}

fn respond_bytes(
    content: ContentDelivery,
    method: &Method,
    data: Bytes,
    content_range: Option<String>,
) -> Response<Body> {
    let status = if content_range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    let len = data.len();
    let body = if *method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(data)
    };
    let mut response = Response::builder()
        .status(status)
        .body(body)
        .expect("content response");
    let (disposition, cache) = meta_parts(&content.meta);
    decorate(
        response.headers_mut(),
        Some(&content.meta.content_type),
        content.meta.etag.as_deref(),
        disposition.as_deref(),
        cache.as_deref(),
    );
    insert(response.headers_mut(), CONTENT_LENGTH, &len.to_string());
    insert(response.headers_mut(), header::ACCEPT_RANGES, "bytes");
    if let Some(content_range) = content_range {
        insert(response.headers_mut(), CONTENT_RANGE, &content_range);
    }
    response
}

fn range_not_satisfiable(total: u64, meta: &DeliveryMeta) -> Response<Body> {
    let mut response = Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .body(Body::empty())
        .expect("416 response");
    decorate(
        response.headers_mut(),
        None,
        meta.etag.as_deref(),
        None,
        Some(&meta.cache.header_value()),
    );
    insert(
        response.headers_mut(),
        CONTENT_RANGE,
        &format!("bytes */{total}"),
    );
    response
}

enum RangeSlice {
    Ignore,
    NotSatisfiable,
    Bytes { start: u64, data: Bytes },
}

fn slice_header(bytes: &Bytes, header: Option<&HeaderValue>) -> RangeSlice {
    let Some(header) = header else {
        return RangeSlice::Ignore;
    };
    let Ok(text) = header.to_str() else {
        return RangeSlice::Ignore;
    };
    let Some(spec) = text.trim().strip_prefix("bytes=") else {
        return RangeSlice::Ignore;
    };
    if spec.is_empty() || spec.contains(',') {
        return RangeSlice::Ignore;
    }
    let total = bytes.len() as u64;
    if let Some(suffix) = spec.strip_prefix('-') {
        let Ok(n) = suffix.parse::<u64>() else {
            return RangeSlice::Ignore;
        };
        if n == 0 || total == 0 {
            return RangeSlice::NotSatisfiable;
        }
        let start = total.saturating_sub(n);
        return RangeSlice::Bytes {
            start,
            data: bytes.slice(start as usize..bytes.len()),
        };
    }
    let Some((start, end)) = spec.split_once('-') else {
        return RangeSlice::Ignore;
    };
    let Ok(start) = start.parse::<u64>() else {
        return RangeSlice::Ignore;
    };
    if total == 0 || start >= total {
        return RangeSlice::NotSatisfiable;
    }
    let end_inclusive = if end.is_empty() {
        total - 1
    } else {
        match end.parse::<u64>() {
            Ok(end) => end,
            Err(_) => return RangeSlice::Ignore,
        }
    };
    if end_inclusive < start {
        return RangeSlice::NotSatisfiable;
    }
    let end_inclusive = end_inclusive.min(total - 1);
    let end_exclusive = (end_inclusive + 1) as usize;
    RangeSlice::Bytes {
        start,
        data: bytes.slice(start as usize..end_exclusive),
    }
}

fn not_modified_since(headers: &HeaderMap, modified: OffsetDateTime) -> bool {
    let Some(value) = headers.get(IF_MODIFIED_SINCE) else {
        return false;
    };
    let Ok(text) = value.to_str() else {
        return false;
    };
    let Ok(since) = httpdate::parse_http_date(text) else {
        return false;
    };
    let modified = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(modified.unix_timestamp().max(0) as u64);
    modified <= since
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::body::to_bytes;
    use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
    use time::OffsetDateTime;

    use super::super::delivery::{
        CachePolicy, ContentDelivery, Delivery, DeliveryMeta, Disposition, LocalFile,
        RedirectDelivery,
    };
    use super::*;

    fn meta(etag: &str, filename: Option<&str>) -> DeliveryMeta {
        DeliveryMeta {
            content_type: "image/png".into(),
            etag: Some(etag.to_owned()),
            disposition: filename.map(|name| Disposition::attachment(name)),
            cache: CachePolicy::private(Duration::from_secs(86_400)),
            last_modified: Some(OffsetDateTime::now_utc() - time::Duration::hours(2)),
        }
    }

    fn content(bytes: &[u8], etag: &str, filename: Option<&str>) -> Delivery {
        Delivery::Content(ContentDelivery {
            bytes: Bytes::copy_from_slice(bytes),
            meta: meta(etag, filename),
        })
    }

    async fn body_of(response: axum::http::Response<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, headers, bytes.to_vec())
    }

    #[tokio::test]
    async fn content_matrix_covers_200_206_416_304_head_and_disposition() {
        let etag = "\"abc\"";
        let bytes = b"abcdefghij";
        let (status, headers, body) = body_of(
            content(bytes, etag, None)
                .respond(&Method::GET, &HeaderMap::new())
                .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, bytes);
        assert_eq!(headers.get(CONTENT_TYPE).unwrap(), "image/png");
        assert_eq!(headers.get(ETAG).unwrap(), etag);
        assert_eq!(
            headers.get(CACHE_CONTROL).unwrap(),
            "private, max-age=86400"
        );
        assert_eq!(headers.get(X_CONTENT_TYPE_OPTIONS).unwrap(), "nosniff");
        assert_eq!(headers.get(CONTENT_LENGTH).unwrap(), "10");

        let mut headers_in = HeaderMap::new();
        headers_in.insert(RANGE, HeaderValue::from_static("bytes=0-3"));
        let (status, headers, body) = body_of(
            content(bytes, etag, None)
                .respond(&Method::GET, &headers_in)
                .await,
        )
        .await;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(body, b"abcd");
        assert_eq!(headers.get(CONTENT_RANGE).unwrap(), "bytes 0-3/10");

        headers_in.insert(RANGE, HeaderValue::from_static("bytes=100-200"));
        let (status, headers, body) = body_of(
            content(bytes, etag, None)
                .respond(&Method::GET, &headers_in)
                .await,
        )
        .await;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
        assert!(body.is_empty());
        assert_eq!(headers.get(CONTENT_RANGE).unwrap(), "bytes */10");

        let mut matched = HeaderMap::new();
        matched.insert(IF_NONE_MATCH, HeaderValue::from_static("\"abc\""));
        let (status, headers, body) = body_of(
            content(bytes, etag, None)
                .respond(&Method::GET, &matched)
                .await,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(body.is_empty());
        assert_eq!(headers.get(ETAG).unwrap(), etag);

        let mut since = HeaderMap::new();
        let http_date = httpdate::fmt_http_date(std::time::SystemTime::now());
        since.insert(
            IF_MODIFIED_SINCE,
            HeaderValue::from_str(&http_date).unwrap(),
        );
        let (status, _, body) = body_of(
            content(bytes, etag, None)
                .respond(&Method::GET, &since)
                .await,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(body.is_empty());

        let (status, headers, body) = body_of(
            content(bytes, etag, None)
                .respond(&Method::HEAD, &HeaderMap::new())
                .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());
        assert_eq!(headers.get(CONTENT_LENGTH).unwrap(), "10");

        let (status, headers, _) = body_of(
            content(bytes, etag, Some("Naïve.png"))
                .respond(&Method::GET, &HeaderMap::new())
                .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let disposition = headers.get(CONTENT_DISPOSITION).unwrap().to_str().unwrap();
        assert!(disposition.contains("filename=\"Na_ve.png\""));
        assert!(disposition.contains("filename*=UTF-8''Na%C3%AFve.png"));
    }

    #[tokio::test]
    async fn etag_match_does_not_open_a_missing_file() {
        let delivery = Delivery::File(LocalFile {
            path: std::path::PathBuf::from("/no/such/rundtisch-object"),
            meta: meta("\"deadbeef\"", None),
        });
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"deadbeef\""));
        let response = delivery.respond(&Method::GET, &headers).await;
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn serve_file_handles_range_head_and_if_modified_since() {
        let dir = std::env::temp_dir().join(format!(
            "rundtisch-http-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("photo.bin");
        std::fs::write(&path, b"abcdefghij").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(48 * 3600);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old)
            .unwrap();

        let file = || {
            Delivery::File(LocalFile {
                path: path.clone(),
                meta: meta("\"file\"", Some("photo.bin")),
            })
        };

        let (status, _, body) =
            body_of(file().respond(&Method::GET, &HeaderMap::new()).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"abcdefghij");

        let mut range = HeaderMap::new();
        range.insert(RANGE, HeaderValue::from_static("bytes=0-3"));
        let (status, headers, body) = body_of(file().respond(&Method::GET, &range).await).await;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(body, b"abcd");
        assert!(headers.get(CONTENT_RANGE).is_some());

        range.insert(RANGE, HeaderValue::from_static("bytes=100-200"));
        let (status, _, _) = body_of(file().respond(&Method::GET, &range).await).await;
        assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);

        let (status, headers, body) =
            body_of(file().respond(&Method::HEAD, &HeaderMap::new()).await).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());
        assert_eq!(headers.get(CONTENT_LENGTH).unwrap(), "10");

        let mut since = HeaderMap::new();
        let http_date = httpdate::fmt_http_date(std::time::SystemTime::now());
        since.insert(
            IF_MODIFIED_SINCE,
            HeaderValue::from_str(&http_date).unwrap(),
        );
        let (status, _, body) = body_of(file().respond(&Method::GET, &since).await).await;
        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert!(body.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn redirect_is_private_and_capped_at_expiry() {
        let delivery = Delivery::Redirect(RedirectDelivery {
            location: "https://example.test/object".into(),
            expires_at: OffsetDateTime::now_utc() + time::Duration::minutes(5),
        });
        let response = delivery.respond(&Method::GET, &HeaderMap::new()).await;
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(LOCATION).unwrap(),
            "https://example.test/object"
        );
        let cache = response
            .headers()
            .get(CACHE_CONTROL)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cache.starts_with("private, max-age="));
        let secs: u64 = cache
            .trim_start_matches("private, max-age=")
            .parse()
            .unwrap();
        assert!(secs <= 300);
    }
}

fn serve_redirect(redirect: RedirectDelivery) -> Response<Body> {
    let now = OffsetDateTime::now_utc();
    let secs = (redirect.expires_at - now).whole_seconds().max(0) as u64;
    let mut response = Response::builder()
        .status(StatusCode::FOUND)
        .body(Body::empty())
        .expect("302 response");
    insert(response.headers_mut(), LOCATION, &redirect.location);
    insert(
        response.headers_mut(),
        CACHE_CONTROL,
        &format!("private, max-age={secs}"),
    );
    response
}
