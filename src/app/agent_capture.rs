//! LCV-145 — answer `capture_canvas` and the pre-upload check (ADR 0011
//! items 1, 2, 6 and 10).
//!
//! Everything happens synchronously, on the UI thread, in the frame that
//! received the `Act`: live permission check, framing from `App::camera`,
//! software raster of the live document, PNG encode, outcome. There is no
//! pending capture, deadline or late event, and no framebuffer is read — the
//! picture is a function of the document and the camera only.

use crate::agent::AgentOutcome;
use crate::app::App;
use crate::app::agent_apply::CAPTURE_DISABLED;
use crate::render::raster::{encode_png_gray, rasterize};

/// Longest edge of a canvas image, in pixels. Never upscaled to reach it.
const MAX_EDGE_PX: f32 = 1024.0;

/// Largest PNG a capture may produce: 2 MiB (ADR 0011 item 5).
const MAX_PNG_BYTES: usize = 2 * 1024 * 1024;

/// The refusal for a viewport with no area (LCV-145 AC 4).
const NO_AREA: &str = "the canvas viewport has no area; nothing to capture";

/// The refusal for an upload the UI no longer authorises (ADR 0011 item 10).
const UPLOAD_REFUSED: &str = "canvas upload not authorised";

/// Both opt-ins, read live.
fn allowed(app: &App) -> bool {
    app.settings.agent_allow_canvas_capture && app.settings.agent_model_supports_vision
}

/// `CaptureCanvas`: the visible viewport, rasterized and encoded, or the
/// pinned refusal. Reads the live settings, never the turn's snapshot.
pub(crate) fn capture(app: &App) -> AgentOutcome {
    if !allowed(app) {
        return AgentOutcome::Refused(CAPTURE_DISABLED.to_owned());
    }
    let [vw, vh] = app.camera.viewport_size_px;
    let Some((w, h)) = pixel_size(vw, vh) else {
        return AgentOutcome::Refused(NO_AREA.to_owned());
    };
    let lo = app.camera.screen_to_world(egui::pos2(0.0, vh));
    let hi = app.camera.screen_to_world(egui::pos2(vw, 0.0));
    let world = [lo.x, lo.y, hi.x, hi.y];
    let pixels = rasterize(&app.document.entities, app.document.bed_mm, world, w, h);
    let png = match encode_png_gray(&pixels, w, h) {
        Ok(png) => png,
        Err(e) => return AgentOutcome::Refused(format!("canvas image could not be encoded: {e}")),
    };
    match check_size(png, MAX_PNG_BYTES) {
        Ok(png) => AgentOutcome::Observed {
            text: outcome_text(w, h, world, app.history.revision()),
            png,
        },
        Err(refusal) => refusal,
    }
}

/// `AuthorizeUpload`: yes only if both opt-ins are still on and the turn's
/// `endpoint` / `model` are still the live ones. A yes leaves a `note` row
/// disclosing the upload; a no leaves nothing (the loop withholds the image).
pub(crate) fn authorize(app: &mut App, endpoint: &str, model: &str) -> AgentOutcome {
    let same_target = app.settings.agent_endpoint == endpoint && app.settings.agent_model == model;
    if !(allowed(app) && same_target) {
        return AgentOutcome::Refused(UPLOAD_REFUSED.to_owned());
    }
    let note = format!("Sending a canvas image to {model}.");
    app.agent.chat.push(("note".to_owned(), note.clone()));
    AgentOutcome::Ok(note)
}

/// The image size for a `vw × vh` px viewport: scaled down uniformly so the
/// longest edge is at most [`MAX_EDGE_PX`], never upscaled. `None` when the
/// viewport has no area (less than one pixel either way, or not finite).
fn pixel_size(vw: f32, vh: f32) -> Option<(u32, u32)> {
    if !(vw >= 1.0 && vh >= 1.0 && vw.is_finite() && vh.is_finite()) {
        return None;
    }
    let scale = (MAX_EDGE_PX / vw.max(vh)).min(1.0);
    // Both products are in 1..=1024 after the checks above.
    let px = |v: f32| (v * scale).round().max(1.0) as u32;
    Some((px(vw), px(vh)))
}

/// Refuse a PNG over `limit` bytes. The limit is injected so the check is
/// testable; a real capture cannot reach 2 MiB (ADR 0011 item 5).
fn check_size(png: Vec<u8>, limit: usize) -> Result<Vec<u8>, AgentOutcome> {
    if png.len() > limit {
        return Err(AgentOutcome::Refused(
            "canvas image exceeds 2 MiB".to_owned(),
        ));
    }
    Ok(png)
}

/// The tool result and transcript row (LCV-145 AC 5), mm to 3 decimals.
fn outcome_text(w: u32, h: u32, world: [f64; 4], revision: u64) -> String {
    let [x0, y0, x1, y1] = world;
    format!(
        "Canvas {w}×{h} px of X {x0:.3}..{x1:.3} mm, Y {y0:.3}..{y1:.3} mm \
         (Y up; bed outline grey, entities black), revision {revision}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    fn app_with(allow: bool, supports: bool, viewport: [f32; 2]) -> App {
        let mut app = App::default();
        app.settings.agent_allow_canvas_capture = allow;
        app.settings.agent_model_supports_vision = supports;
        app.camera.viewport_size_px = viewport;
        app
    }

    #[test]
    fn pixel_size_scales_down_to_1024_and_never_up() {
        assert_eq!(pixel_size(1600.0, 900.0), Some((1024, 576)));
        assert_eq!(pixel_size(900.0, 1600.0), Some((576, 1024)));
        assert_eq!(pixel_size(800.0, 600.0), Some((800, 600)));
        assert_eq!(pixel_size(1024.0, 1024.0), Some((1024, 1024)));
        assert_eq!(pixel_size(1025.0, 1.0), Some((1024, 1)));
        assert_eq!(pixel_size(1.0, 1.0), Some((1, 1)));
        for (w, h) in [(0.0, 600.0), (600.0, 0.0), (0.5, 600.0), (f32::NAN, 600.0)] {
            assert_eq!(pixel_size(w, h), None, "{w}×{h}");
        }
        assert_eq!(pixel_size(f32::INFINITY, 600.0), None);
    }

    #[test]
    fn a_zero_area_viewport_gets_the_pinned_refusal() {
        let app = app_with(true, true, [0.0, 600.0]);
        assert_eq!(
            capture(&app),
            AgentOutcome::Refused("the canvas viewport has no area; nothing to capture".into())
        );
    }

    #[test]
    fn check_size_refuses_only_above_the_limit() {
        assert_eq!(check_size(vec![0; 10], 10), Ok(vec![0; 10]));
        assert_eq!(
            check_size(vec![0; 11], 10),
            Err(AgentOutcome::Refused("canvas image exceeds 2 MiB".into()))
        );
        assert_eq!(MAX_PNG_BYTES, 2_097_152);
    }

    #[test]
    fn the_outcome_string_is_pinned_character_for_character() {
        assert_eq!(
            outcome_text(800, 600, [-1.0, 2.5, 399.0, 302.5], 7),
            "Canvas 800×600 px of X -1.000..399.000 mm, Y 2.500..302.500 mm \
             (Y up; bed outline grey, entities black), revision 7."
        );
    }

    #[test]
    fn a_capture_frames_the_viewport_and_reports_it() {
        let mut app = app_with(true, true, [800.0, 600.0]);
        app.camera.center_world = Vec2::new(200.0, 150.0);
        app.camera.mm_per_px = 0.5;
        let AgentOutcome::Observed { text, png } = capture(&app) else {
            panic!("expected Observed");
        };
        assert_eq!(
            text,
            "Canvas 800×600 px of X 0.000..400.000 mm, Y 0.000..300.000 mm \
             (Y up; bed outline grey, entities black), revision 0."
        );
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn a_live_setting_off_gets_the_pinned_refusal() {
        for (allow, supports) in [(false, false), (true, false), (false, true)] {
            let app = app_with(allow, supports, [800.0, 600.0]);
            assert_eq!(
                capture(&app),
                AgentOutcome::Refused("canvas capture is disabled in Agent settings".into()),
                "({allow}, {supports})"
            );
        }
    }

    #[test]
    fn authorize_says_yes_only_for_both_flags_and_the_same_target() {
        let base = app_with(true, true, [800.0, 600.0]);
        let (endpoint, model) = (
            base.settings.agent_endpoint.clone(),
            base.settings.agent_model.clone(),
        );
        let mut app = app_with(true, true, [800.0, 600.0]);
        let yes = authorize(&mut app, &endpoint, &model);
        let note = format!("Sending a canvas image to {model}.");
        assert_eq!(yes, AgentOutcome::Ok(note.clone()));
        assert_eq!(app.agent.chat.last(), Some(&("note".to_owned(), note)));

        let cases: [(bool, bool, &str, &str); 5] = [
            (false, true, &endpoint, &model),
            (true, false, &endpoint, &model),
            (true, true, "https://elsewhere.invalid", &model),
            (true, true, &endpoint, "other/model"),
            (false, false, &endpoint, &model),
        ];
        for (allow, supports, e, m) in cases {
            let mut app = app_with(allow, supports, [800.0, 600.0]);
            let rows = app.agent.chat.len();
            assert_eq!(
                authorize(&mut app, e, m),
                AgentOutcome::Refused("canvas upload not authorised".into()),
                "({allow}, {supports}, {e}, {m})"
            );
            assert_eq!(app.agent.chat.len(), rows, "a no leaves no row");
        }
    }
}
