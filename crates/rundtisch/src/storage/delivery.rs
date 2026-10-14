use bytes::Bytes;
use time::OffsetDateTime;

/// How the client should cache the response.
#[derive(Clone, Debug)]
pub enum CachePolicy {
    Private { max_age: std::time::Duration },
    NoStore,
}

impl CachePolicy {
    pub fn private(max_age: std::time::Duration) -> Self {
        Self::Private { max_age }
    }

    pub fn header_value(&self) -> String {
        match self {
            CachePolicy::Private { max_age } => format!("private, max-age={}", max_age.as_secs()),
            CachePolicy::NoStore => "no-store".to_owned(),
        }
    }
}

/// `Content-Disposition` for a delivered object.
#[derive(Clone, Debug)]
pub enum Disposition {
    Inline { filename: Option<String> },
    Attachment { filename: Option<String> },
}

impl Disposition {
    pub fn inline() -> Self {
        Self::Inline { filename: None }
    }

    pub fn attachment(filename: impl Into<String>) -> Self {
        Self::Attachment {
            filename: Some(filename.into()),
        }
    }

    /// Attachment named with the object's original filename, when it has one.
    pub fn attachment_from(object: &super::ObjectInfo) -> Self {
        Self::Attachment {
            filename: object.original_filename.clone(),
        }
    }

    pub fn header_value(&self) -> String {
        match self {
            Disposition::Inline { filename } => format_disposition("inline", filename.as_deref()),
            Disposition::Attachment { filename } => {
                format_disposition("attachment", filename.as_deref())
            }
        }
    }
}

fn format_disposition(kind: &str, filename: Option<&str>) -> String {
    let Some(filename) = filename.filter(|value| !value.is_empty()) else {
        return kind.to_owned();
    };
    let ascii: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii() && c != '"' && c != '\\' && !c.is_control() {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut encoded = String::new();
    for byte in filename.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    format!("{kind}; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

/// Options the application sets. The store overwrites content type and ETag from the row.
#[derive(Clone, Debug)]
pub struct DeliverOptions {
    pub disposition: Option<Disposition>,
    pub cache: CachePolicy,
    pub(crate) content_type: String,
    pub(crate) etag: Option<String>,
    pub(crate) last_modified: Option<OffsetDateTime>,
}

impl DeliverOptions {
    /// Content type, ETag, and last-modified are filled from the committed row.
    pub fn new(disposition: Option<Disposition>, cache: CachePolicy) -> Self {
        Self {
            disposition,
            cache,
            content_type: String::new(),
            etag: None,
            last_modified: None,
        }
    }
}

impl Default for DeliverOptions {
    fn default() -> Self {
        Self::new(
            None,
            CachePolicy::private(std::time::Duration::from_secs(86_400)),
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeliveryMeta {
    pub(crate) content_type: String,
    pub(crate) etag: Option<String>,
    pub(crate) disposition: Option<Disposition>,
    pub(crate) cache: CachePolicy,
    pub(crate) last_modified: Option<OffsetDateTime>,
}

impl DeliveryMeta {
    pub(crate) fn from_options(opts: &DeliverOptions) -> Self {
        Self {
            content_type: opts.content_type.clone(),
            etag: opts.etag.clone(),
            disposition: opts.disposition.clone(),
            cache: opts.cache.clone(),
            last_modified: opts.last_modified,
        }
    }
}

/// A local path. The path stays crate-private; callers use [`Delivery::respond`].
pub struct LocalFile {
    pub(crate) path: std::path::PathBuf,
    pub(crate) meta: DeliveryMeta,
}

pub struct ContentDelivery {
    pub(crate) bytes: Bytes,
    pub(crate) meta: DeliveryMeta,
}

pub struct RedirectDelivery {
    pub(crate) location: String,
    pub(crate) expires_at: OffsetDateTime,
}

/// How an adapter wants the response sent.
///
/// `File` delegates to tower-http `ServeFile` (range, 304 from mtime, HEAD).
/// `Content` is an in-memory body. `Redirect` is reserved for a later presigned GET.
pub enum Delivery {
    File(LocalFile),
    Content(ContentDelivery),
    Redirect(RedirectDelivery),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposition_encodes_non_ascii_filenames() {
        let header = Disposition::attachment("Naïve.png").header_value();
        assert!(header.starts_with("attachment; "));
        assert!(header.contains("filename=\"Na_ve.png\""));
        assert!(header.contains("filename*=UTF-8''Na%C3%AFve.png"));
        assert_eq!(Disposition::inline().header_value(), "inline");
    }
}
