//! Unit tests for [`crate::app`] (extracted from `app.rs` — LCV-053).

use super::*;
use crate::geometry::Vec2;
use crate::render::Camera;

/// LCV-030 AC#1 — `App::default()` produces an empty document and an
/// empty history.
#[test]
fn app_default_constructs_with_empty_document_and_history() {
    let app = App::default();
    assert_eq!(app.document.entity_count(), 0);
    assert!(!app.history.can_undo());
    assert!(!app.history.can_redo());
}

/// LCV-031 AC#13 — `App` carries a `Camera` field and it defaults to
/// [`Camera::default()`].
#[test]
fn app_default_camera_matches_camera_default() {
    let app = App::default();
    assert_eq!(app.camera, Camera::default());
    assert_eq!(app.camera.mm_per_px, 1.0);
}

/// LCV-032 AC#1 — `App` carries `last_cursor_world` defaulting to `None`.
#[test]
fn app_default_has_no_cursor_world() {
    let app = App::default();
    assert_eq!(app.last_cursor_world, None);
}

/// LCV-034 AC#7 — `App` carries a `Bed` field that defaults to
/// [`crate::render::Bed::default()`].
#[test]
fn app_default_bed_matches_bed_default() {
    let app = App::default();
    assert_eq!(app.bed, crate::render::Bed::default());
    assert_eq!(app.bed.size_mm, [400.0, 400.0]);
}

/// LCV-032 AC#8 — wheel zoom helper with positive factor zooms in.
#[test]
fn wheel_zoom_dispatch_positive_scroll_zooms_in() {
    let mut cam = Camera {
        center_world: Vec2::new(0.0, 0.0),
        mm_per_px: 1.0,
        viewport_size_px: [800.0, 600.0],
    };
    handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.1);
    assert!(cam.mm_per_px < 1.0);
}

/// LCV-032 AC#8 — wheel zoom helper with factor < 1 zooms out.
#[test]
fn wheel_zoom_dispatch_negative_scroll_zooms_out() {
    let mut cam = Camera {
        center_world: Vec2::new(0.0, 0.0),
        mm_per_px: 1.0,
        viewport_size_px: [800.0, 600.0],
    };
    handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.0 / 1.1);
    assert!(cam.mm_per_px > 1.0);
}

/// LCV-032 AC#8 — wheel zoom helper with factor 1.0 is a no-op.
#[test]
fn wheel_zoom_dispatch_unity_scroll_is_noop() {
    let mut cam = Camera {
        center_world: Vec2::new(0.0, 0.0),
        mm_per_px: 1.0,
        viewport_size_px: [800.0, 600.0],
    };
    handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.0);
    assert!((cam.mm_per_px - 1.0).abs() < 1e-12);
}

/// LCV-032 AC#8 — zoom extents dispatch resets on empty document.
#[test]
fn zoom_extents_dispatch_on_empty_document_resets() {
    let mut cam = Camera {
        center_world: Vec2::new(7.0, -3.0),
        mm_per_px: 4.0,
        viewport_size_px: [10.0, 10.0],
    };
    let doc = crate::document::Document::default();
    handle_zoom_extents(&mut cam, &doc, [800.0, 600.0]);
    assert_eq!(cam.center_world, Vec2::new(0.0, 0.0));
    assert_eq!(cam.mm_per_px, 1.0);
    assert_eq!(cam.viewport_size_px, [800.0, 600.0]);
}

/// LCV-037 AC#7 — `App` carries `preview_entities` defaulting to empty.
#[test]
fn app_default_has_empty_preview_entities() {
    let app = App::default();
    assert!(app.preview_entities.is_empty());
}

/// LCV-058 AC#10 — `App` carries `settings` defaulting to `Settings::default()`.
#[test]
fn app_default_settings_equals_settings_default() {
    let app = App::default();
    assert_eq!(app.settings, crate::io::settings::Settings::default());
}

/// LCV-069 AC#7 / §5 — `App::default().about_open` is `false`.
#[test]
fn app_default_about_open_is_false() {
    let app = App::default();
    assert!(!app.about_open);
}

/// LCV-068 AC#3 — `App::default().command_line_input` is the empty string.
#[test]
fn app_default_command_line_input_is_empty() {
    let app = App::default();
    assert!(app.command_line_input.is_empty());
}

/// LCV-076 AC#9 — `App::default().agent_settings_open` is `false`.
#[test]
fn app_default_agent_settings_open_is_false() {
    let app = App::default();
    assert!(!app.agent_settings_open);
}

/// LCV-053 AC#1 — `App::default().ortho` is `false`.
#[test]
fn app_default_ortho_is_false() {
    let app = App::default();
    assert!(!app.ortho_enabled);
}
