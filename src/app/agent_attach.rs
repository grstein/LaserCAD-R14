//! LCV-199 — the agent's reference image, UI side: [`Attachment`],
//! [`attach_image`], [`poll_attach_request`] and [`take_for_send`].
//!
//! The panel only raises `agent.attach_requested`; the frame wiring turns it
//! into one native picker through `io/dialogs.rs` (ADR 0005) and feeds the
//! chosen path to [`attach_image`] — the seam tests call directly. The file is
//! checked when attached and read again, and re-checked, when the prompt is
//! sent: the bytes cross into the worker as a `UserImage` (ADR 0007 §D1).

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::agent::attachment::{ImageKind, MAX_IMAGE_BYTES, UserImage, check};
use crate::app::{App, Severity};

/// The image the operator attached to the next prompt (LCV-199).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// Where the file is; read again at send time.
    pub path: PathBuf,
    /// The file name the chip and the transcript show.
    pub name: String,
    /// The format its first bytes had when it was attached.
    pub kind: ImageKind,
}

/// The file at `path`, read at most one byte past [`MAX_IMAGE_BYTES`]: enough
/// for [`check`] to see it is too large without reading all of it.
fn read_capped(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let limit = u64::try_from(MAX_IMAGE_BYTES).unwrap_or(u64::MAX) + 1;
    std::fs::File::open(path)?
        .take(limit)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The file name of `path`, or the whole path when it has none.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Attach the image at `path` to the next prompt (AC 1, AC 4). A file that
/// cannot be read, is not a PNG or JPEG by its first bytes, or is over 2 MB
/// is refused with a warning naming the reason, and attaches nothing.
pub fn attach_image(app: &mut App, path: &Path) {
    let name = file_name(path);
    let checked = read_capped(path)
        .map_err(|e| format!("Image not attached: {name} could not be read: {e}."))
        .and_then(|bytes| check(&name, &bytes));
    match checked {
        Ok(kind) => {
            app.agent.attachment = Some(Attachment {
                path: path.to_owned(),
                name,
                kind,
            });
        }
        Err(message) => app.say(Severity::Warning, message),
    }
}

/// Consume the panel's `Attach image…` request: one native picker, whose
/// choice goes to [`attach_image`]. Called by the frame wiring after the
/// panel is drawn; never by a test (ADR 0005).
#[expect(dead_code, reason = "LCV-199 T8 wires it into panels.rs")]
pub(crate) fn poll_attach_request(app: &mut App) {
    if std::mem::take(&mut app.agent.attach_requested)
        && let Some(path) = crate::io::dialogs::pick_image_dialog()
    {
        attach_image(app, &path);
    }
}

/// The attached image for the turn being sent, read and checked again (AC 2,
/// AC 5); `Ok(None)` when nothing is attached. On success the attachment is
/// cleared.
///
/// # Errors
///
/// The transcript row saying why nothing was sent: the file cannot be read
/// or is no longer a PNG or JPEG under 2 MB, or `Model supports images` was
/// turned off after attaching. The attachment is kept.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "LCV-199 T7 wires it into start_turn")
)]
pub(crate) fn take_for_send(app: &mut App) -> Result<Option<UserImage>, String> {
    let Some(attachment) = &app.agent.attachment else {
        return Ok(None);
    };
    if !app.settings.agent_model_supports_vision {
        return Err("\"Model supports images\" is off. Nothing was sent.".to_owned());
    }
    let name = attachment.name.clone();
    let bytes = read_capped(&attachment.path)
        .map_err(|e| format!("Image {name} could not be read: {e}. Nothing was sent."))?;
    let kind = check(&name, &bytes).map_err(|reason| format!("{reason} Nothing was sent."))?;
    app.agent.attachment = None;
    Ok(Some(UserImage { name, kind, bytes }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

    /// A fresh file `name` holding `bytes` in a per-test scratch directory.
    fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lcv199_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn vision_app() -> App {
        let mut app = App::default();
        app.settings.agent_model_supports_vision = true;
        app
    }

    #[test]
    fn a_png_attaches_and_is_taken_once() {
        let mut app = vision_app();
        let path = scratch("unit_ok.png", PNG);
        attach_image(&mut app, &path);
        let attached = app.agent.attachment.clone().unwrap();
        assert_eq!(
            (attached.name.as_str(), attached.kind),
            ("unit_ok.png", ImageKind::Png)
        );
        let image = take_for_send(&mut app).unwrap().unwrap();
        assert_eq!((image.kind, image.bytes.as_slice()), (ImageKind::Png, PNG));
        assert!(app.agent.attachment.is_none());
        assert_eq!(take_for_send(&mut app), Ok(None));
    }

    #[test]
    fn a_missing_file_attaches_nothing_and_says_why() {
        let mut app = vision_app();
        attach_image(&mut app, Path::new("/nonexistent/lcv199/x.png"));
        assert!(app.agent.attachment.is_none());
        assert!(
            app.command_feedback
                .starts_with("Image not attached: x.png could not be read:")
        );
        assert_eq!(app.command_feedback_severity, Severity::Warning);
    }

    #[test]
    fn a_refused_send_keeps_the_attachment() {
        let mut app = vision_app();
        let path = scratch("unit_gone.png", PNG);
        attach_image(&mut app, &path);
        app.settings.agent_model_supports_vision = false;
        let off = take_for_send(&mut app).unwrap_err();
        assert_eq!(off, "\"Model supports images\" is off. Nothing was sent.");
        app.settings.agent_model_supports_vision = true;
        std::fs::remove_file(&path).unwrap();
        let gone = take_for_send(&mut app).unwrap_err();
        assert!(
            gone.starts_with("Image unit_gone.png could not be read: "),
            "{gone}"
        );
        assert!(gone.ends_with(". Nothing was sent."), "{gone}");
        assert!(app.agent.attachment.is_some());
    }

    #[test]
    fn a_file_grown_past_the_cap_is_refused_at_send() {
        let mut app = vision_app();
        let path = scratch("unit_grown.png", PNG);
        attach_image(&mut app, &path);
        let mut big = PNG.to_vec();
        big.resize(MAX_IMAGE_BYTES + 1, 0);
        std::fs::write(&path, big).unwrap();
        assert_eq!(
            take_for_send(&mut app),
            Err("Image not attached: unit_grown.png is over 2 MB. Nothing was sent.".to_owned())
        );
    }
}
