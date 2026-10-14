use super::id::new_uuid_v4;

const MAX_KEY_LEN: usize = 512;
const MAX_PREFIX_SEGMENTS: usize = 4;
const MAX_KEY_SEGMENTS: usize = 8;
const MAX_PREFIX_SEGMENT_LEN: usize = 64;
const MAX_KEY_SEGMENT_LEN: usize = 200;

/// Optional key prefix supplied by the app. The library still chooses the object name.
///
/// Up to four segments of `[A-Za-z0-9._-]`. A segment cannot be `.`, `..`, or start
/// with `.`. Crockis passes the hyphenated `collections.storage_folder`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectPrefix {
    raw: String,
}

impl ObjectPrefix {
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            return Err("prefix is empty".into());
        }
        let segments: Vec<&str> = value.split('/').collect();
        if segments.len() > MAX_PREFIX_SEGMENTS {
            return Err("prefix has more than 4 segments".into());
        }
        for segment in segments {
            validate_segment(segment, MAX_PREFIX_SEGMENT_LEN)?;
        }
        if value.len() > MAX_KEY_LEN {
            return Err("prefix is too long".into());
        }
        Ok(Self {
            raw: value.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

/// Library-generated relative key: `{prefix/}{uuid}[.{ext}]`. Never sent to clients.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectKey(String);

impl ObjectKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        if value.is_empty() || value.len() > MAX_KEY_LEN {
            return Err("storage key length is invalid".into());
        }
        let segments: Vec<&str> = value.split('/').collect();
        if segments.len() > MAX_KEY_SEGMENTS {
            return Err("storage key has too many segments".into());
        }
        for segment in segments {
            validate_segment(segment, MAX_KEY_SEGMENT_LEN)?;
        }
        Ok(Self(value.to_owned()))
    }

    pub fn is_temporary(&self) -> bool {
        self.0.ends_with(".partial")
    }
}

impl std::fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `{prefix/}{uuid}[.{ext}]`. The uuid is a fresh v4, separate from `public_id`.
pub(crate) fn generate_key(prefix: Option<&ObjectPrefix>, content_type: &str) -> ObjectKey {
    let id = new_uuid_v4();
    let name = match extension_for_content_type(content_type) {
        Some(ext) => format!("{id}.{ext}"),
        None => id.to_string(),
    };
    let raw = match prefix {
        Some(prefix) => format!("{}/{name}", prefix.as_str()),
        None => name,
    };
    ObjectKey::parse(&raw).expect("generated storage key")
}

/// Extension used only so a human browsing the volume can recognize the file.
/// Serving uses the stored content type, not this suffix.
pub fn extension_for_content_type(content_type: &str) -> Option<&'static str> {
    let base = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim();
    match base {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "image/avif" => Some("avif"),
        "image/heic" => Some("heic"),
        "image/heif" => Some("heif"),
        _ => None,
    }
}

fn validate_segment(segment: &str, max_len: usize) -> Result<(), String> {
    if segment.is_empty() {
        return Err("path segment is empty".into());
    }
    if segment.len() > max_len {
        return Err("path segment is too long".into());
    }
    if segment == "." || segment == ".." || segment.starts_with('.') {
        return Err(format!("path segment {segment:?} is not allowed"));
    }
    if !segment
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(format!("path segment {segment:?} has invalid characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_rules() {
        let folder = "018f5c20-0000-7000-8000-000000000001";
        assert_eq!(ObjectPrefix::parse(folder).unwrap().as_str(), folder);
        assert!(ObjectPrefix::parse("").is_err());
        assert!(ObjectPrefix::parse(".").is_err());
        assert!(ObjectPrefix::parse("..").is_err());
        assert!(ObjectPrefix::parse(".hidden").is_err());
        assert!(ObjectPrefix::parse("a/../b").is_err());
        assert!(ObjectPrefix::parse("a//b").is_err());
        assert!(ObjectPrefix::parse("a/b/c/d/e").is_err());
        assert!(ObjectPrefix::parse("a/b c").is_err());
        assert!(ObjectPrefix::parse("photos/2024").is_ok());
    }

    #[test]
    fn generated_key_uses_prefix_extension_and_a_fresh_uuid() {
        let prefix = ObjectPrefix::parse("018f5c20-0000-7000-8000-000000000001").unwrap();
        let key = generate_key(Some(&prefix), "image/png");
        assert!(key.as_str().starts_with(&format!("{}/", prefix.as_str())));
        assert!(key.as_str().ends_with(".png"));
        assert!(!key.is_temporary());
        let other = generate_key(Some(&prefix), "text/plain");
        assert_ne!(key, other);
        assert!(!other.as_str().contains('.'));
        assert_eq!(
            extension_for_content_type("image/jpeg; charset=binary"),
            Some("jpg")
        );
        assert_eq!(extension_for_content_type("application/octet-stream"), None);
    }
}
