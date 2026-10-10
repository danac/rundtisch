use std::path::{Path, PathBuf};

use imagesize::{ImageType, blob_size, image_type};
use uuid::Uuid;

pub const DEFAULT_DATA_DIR: &str = "/data";

/// Directory that holds stored image files.
///
/// Wasmer Edge mounts the `data` volume here. `DATA_DIR` overrides that for
/// local migrate and `crockis-native`.
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

pub fn seed_enabled_from(value: Option<&str>) -> bool {
    match value.map(str::trim) {
        None => true,
        Some("") => true,
        Some(value) => !matches!(
            value.to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no" | "skip"
        ),
    }
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

/// Raw 32-byte BLAKE3 digest of file contents.
pub fn checksum_blake3(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
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

pub struct ImageInfo {
    pub width: i32,
    pub height: i32,
    pub extension: &'static str,
}

pub fn image_info(bytes: &[u8], original_filename: &str) -> Result<ImageInfo, String> {
    let kind = image_type(bytes).map_err(|err| err.to_string())?;
    let size = blob_size(bytes).map_err(|err| err.to_string())?;
    let extension = match kind {
        ImageType::Jpeg => {
            if original_filename.to_ascii_lowercase().ends_with(".jpeg") {
                "jpeg"
            } else {
                "jpg"
            }
        }
        ImageType::Png => "png",
        ImageType::Gif => "gif",
        ImageType::Webp => "webp",
        ImageType::Heif(_) => "avif",
        other => return Err(format!("unsupported image type {other:?}")),
    };
    let width = i32::try_from(size.width).map_err(|_| "width does not fit in i32".to_string())?;
    let height =
        i32::try_from(size.height).map_err(|_| "height does not fit in i32".to_string())?;
    if width <= 0 || height <= 0 {
        return Err("image has no pixels".to_string());
    }
    Ok(ImageInfo {
        width,
        height,
        extension,
    })
}

pub fn filename_with_extension(original: &str, extension: &str) -> String {
    let stem = match original.rsplit_once('.') {
        Some((stem, existing)) if is_image_extension(existing) => stem,
        _ => original,
    };
    let stem = stem.trim();
    let stem = if stem.is_empty() { "photo" } else { stem };
    format!("{stem}.{extension}")
}

#[cfg(test)]
pub(crate) const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xFE, 0xD6, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "avif"
    )
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
    fn seed_flag_defaults_on() {
        assert!(seed_enabled_from(None));
        assert!(seed_enabled_from(Some("1")));
        assert!(!seed_enabled_from(Some("0")));
        assert!(!seed_enabled_from(Some("skip")));
        assert!(!seed_enabled_from(Some("FALSE")));
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
        assert_eq!(
            checksum_blake3(b"crockis"),
            *blake3::hash(b"crockis").as_bytes()
        );
    }

    #[test]
    fn png_dimensions_and_filename() {
        let info = image_info(super::PNG_1X1, "Stones.jpg").unwrap();
        assert_eq!(info.width, 1);
        assert_eq!(info.height, 1);
        assert_eq!(info.extension, "png");
        assert_eq!(filename_with_extension("Stones.jpg", "png"), "Stones.png");
        assert_eq!(
            filename_with_extension("pexels-photo-3764568.jpeg", "jpeg"),
            "pexels-photo-3764568.jpeg"
        );
        assert_eq!(content_type_for_storage_name("a.png"), "image/png");
    }
}
