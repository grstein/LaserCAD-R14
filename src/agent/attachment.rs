//! LCV-199 — the agent's reference image: what a file must be to ride a
//! prompt, and the owned bytes that cross into the worker.
//!
//! The image is identified by its first bytes, never by its extension, and
//! sent as is: no decoding, no resizing (LCV-199 decisions). The operator's
//! explicit attach is the upload consent; the `data:` URL is built in
//! [`crate::agent::wire`], the only file that encodes one (ADR 0011).
//!
//! Kernel-pure (AGENTS.md §Purity rule): `std` only.

/// The largest image accepted, in bytes: 2 MB, inclusive (AC 4).
pub const MAX_IMAGE_BYTES: usize = 2 * 1024 * 1024;

/// What the attached image becomes once it has ridden its one request, in
/// the turn and in memory alike (AC 6).
pub const ATTACHED_IMAGE_ELIDED: &str = "image elided";

/// The PNG signature.
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The JPEG start-of-image marker followed by the next marker's `0xFF`.
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];

/// The two image formats an attachment may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// Portable Network Graphics.
    Png,
    /// JPEG / JFIF / EXIF.
    Jpeg,
}

impl ImageKind {
    /// The MIME type the image is sent as.
    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
        }
    }
}

/// The format `bytes` start with, if it is PNG or JPEG.
pub fn sniff(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(PNG_MAGIC) {
        Some(ImageKind::Png)
    } else if bytes.starts_with(JPEG_MAGIC) {
        Some(ImageKind::Jpeg)
    } else {
        None
    }
}

/// Check the file `name` holding `bytes` (AC 4).
///
/// # Errors
///
/// The status message naming the reason: not a PNG or JPEG by its first
/// bytes, or over [`MAX_IMAGE_BYTES`].
pub fn check(name: &str, bytes: &[u8]) -> Result<ImageKind, String> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!("Image not attached: {name} is over 2 MB."));
    }
    sniff(bytes).ok_or_else(|| format!("Image not attached: {name} is not a PNG or JPEG."))
}

/// The attached image a turn sends with its user message: checked bytes and
/// the file name. Its `Debug` prints the name, kind and byte count only.
#[derive(Clone, PartialEq, Eq)]
pub struct UserImage {
    /// The file name, as the transcript shows it.
    pub name: String,
    /// The format, by its first bytes.
    pub kind: ImageKind,
    /// The file, as read.
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for UserImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserImage")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("bytes_len", &self.bytes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(len: usize) -> Vec<u8> {
        let mut bytes = PNG_MAGIC.to_vec();
        bytes.resize(len, 0);
        bytes
    }

    #[test]
    fn png_and_jpeg_are_sniffed_by_their_first_bytes() {
        assert_eq!(sniff(&png(8)), Some(ImageKind::Png));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(ImageKind::Jpeg));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF]), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::Png.mime(), "image/png");
        assert_eq!(ImageKind::Jpeg.mime(), "image/jpeg");
    }

    #[test]
    fn a_truncated_or_foreign_header_is_not_an_image() {
        assert_eq!(sniff(&PNG_MAGIC[..7]), None);
        assert_eq!(sniff(&[0xFF, 0xD8]), None);
        assert_eq!(sniff(&[0xFF, 0xD8, 0x00]), None);
        assert_eq!(sniff(b"GIF89a"), None);
        assert_eq!(sniff(&[]), None);
    }

    /// AC 4 — a GIF is refused by name and reason.
    #[test]
    fn a_gif_is_refused() {
        assert_eq!(
            check("cat.gif", b"GIF89a\x01\x00"),
            Err("Image not attached: cat.gif is not a PNG or JPEG.".to_owned())
        );
    }

    /// AC 4 — 2 MB exactly is accepted; one byte more is refused.
    #[test]
    fn two_megabytes_exactly_is_the_largest_accepted() {
        assert_eq!(check("a.png", &png(MAX_IMAGE_BYTES)), Ok(ImageKind::Png));
        assert_eq!(
            check("a.png", &png(MAX_IMAGE_BYTES + 1)),
            Err("Image not attached: a.png is over 2 MB.".to_owned())
        );
        assert_eq!(MAX_IMAGE_BYTES, 2_097_152);
    }

    #[test]
    fn debug_prints_no_bytes() {
        let image = UserImage {
            name: "s.png".into(),
            kind: ImageKind::Png,
            bytes: vec![0xAB; 3],
        };
        let shown = format!("{image:?}");
        assert!(shown.contains("s.png") && shown.contains("bytes_len: 3"));
        assert!(!shown.contains("171"), "{shown}");
    }

    #[test]
    fn attachment_is_kernel_pure() {
        let src = include_str!("attachment.rs");
        let at = src.find("\n#[cfg(test)]").expect("a bare marker");
        for forbidden in [
            concat!("eg", "ui"),
            concat!("ef", "rame"),
            concat!("rf", "d::"),
            concat!("base", "64"),
            concat!("crate::", "app"),
        ] {
            assert!(!src[..at].contains(forbidden), "{forbidden}");
        }
    }
}
