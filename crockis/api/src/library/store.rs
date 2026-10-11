use std::path::{Path, PathBuf};

use uuid::Uuid;

pub const DEFAULT_DATA_DIR: &str = "/data";

/// Directory that holds stored image files.
///
/// Wasmer Edge mounts the `data` volume here. `DATA_DIR` overrides that for
/// local migrate, `crockis-seed`, and `crockis-native`.
pub fn data_dir() -> PathBuf {
    resolve_data_dir(std::env::var("DATA_DIR").ok().as_deref())
}

pub fn resolve_data_dir(override_dir: Option<&str>) -> PathBuf {
    override_dir
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR))
}

/// Join a collection folder and storage file name onto the data directory.
///
/// Files live at `{dir}/{storage_folder}/{filename}`. The folder is the
/// hyphenated collection `storage_folder` UUID. The file name must be a
/// single path segment of ASCII letters, digits, `-`, and `.`, so a row
/// cannot escape the data directory.
pub fn storage_path(dir: &Path, storage_folder: Uuid, filename: &str) -> Option<PathBuf> {
    if !safe_storage_filename(filename) {
        return None;
    }
    let folder = storage_folder.as_hyphenated().to_string();
    Some(dir.join(folder).join(filename))
}

fn safe_storage_filename(filename: &str) -> bool {
    if filename.is_empty() || filename.len() > 255 || filename.starts_with('.') {
        return false;
    }
    if filename.contains("..") || !filename.contains('.') {
        return false;
    }
    filename
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

pub fn content_type_for_storage_name(filename: &str) -> &'static str {
    match storage_extension(filename) {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

pub fn storage_extension(filename: &str) -> &str {
    filename
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or("bin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_data_dir_is_the_volume_mount() {
        assert_eq!(resolve_data_dir(None), PathBuf::from("/data"));
        assert_eq!(resolve_data_dir(Some("  ")), PathBuf::from("/data"));
        assert_eq!(
            resolve_data_dir(Some(" crockis/data ")),
            PathBuf::from("crockis/data")
        );
    }

    #[test]
    fn storage_path_rejects_escape() {
        let dir = Path::new("/data");
        let folder = Uuid::from_u128(0x018f_5c20_0000_7000_8000_000000000001);
        assert_eq!(
            storage_path(dir, folder, "6f0d2c3a-1b2c-4d5e-8f90-aabbccddeeff.jpg"),
            Some(PathBuf::from(
                "/data/018f5c20-0000-7000-8000-000000000001/6f0d2c3a-1b2c-4d5e-8f90-aabbccddeeff.jpg"
            ))
        );
        assert!(storage_path(dir, folder, "../etc/passwd").is_none());
        assert!(storage_path(dir, folder, "a/b.jpg").is_none());
        assert!(storage_path(dir, folder, ".hidden.jpg").is_none());
        assert_eq!(content_type_for_storage_name("a.png"), "image/png");
    }
}
