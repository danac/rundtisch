//! Pixel size of a catalog image. Compiled for `crockis-seed` and for tests,
//! not for the server. Digests are computed by the object store.

use imagesize::{ImageType, blob_size, image_type};

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
    fn png_dimensions_filename_and_checksum() {
        let info = image_info(PNG_1X1, "Stones.jpg").unwrap();
        assert_eq!(info.width, 1);
        assert_eq!(info.height, 1);
        assert_eq!(info.extension, "png");
        assert_eq!(filename_with_extension("Stones.jpg", "png"), "Stones.png");
        assert_eq!(
            filename_with_extension("pexels-photo-3764568.jpeg", "jpeg"),
            "pexels-photo-3764568.jpeg"
        );
    }
}
