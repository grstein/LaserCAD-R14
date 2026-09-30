//! LCV-165 AC 1 — the dock paints a message by its severity: an error in
//! `status.error`, a refusal in `status.warning`, a result in `text.primary`.
//!
//! Colours are read off the painted text shapes (`tests/harness/paint.rs`
//! style), severities off `App::command_feedback_severity`.

use crate::harness;

use harness::{frame, raw_input, submit_command};
use lasercad::app::{App, Severity};

/// `status.error` (DESIGN.md §3).
const STATUS_ERROR: egui::Color32 = egui::Color32::from_rgb(0xff, 0x6b, 0x6b);
/// `status.warning` (DESIGN.md §3).
const STATUS_WARNING: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8f, 0x00);
/// `text.primary` (DESIGN.md §3).
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xd0, 0xd0, 0xd0);

fn boot() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

/// Every text shape of one frame, `Shape::Vec` flattened.
fn text_shapes(ctx: &egui::Context, app: &mut App) -> Vec<egui::epaint::TextShape> {
    frame(ctx, app, vec![]);
    let out = ctx.run(raw_input(Vec::new()), |c| app.update_ui(c));
    let mut stack: Vec<egui::Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
    let mut found = Vec::new();
    while let Some(shape) = stack.pop() {
        match shape {
            egui::Shape::Vec(inner) => stack.extend(inner),
            egui::Shape::Text(t) => found.push(t),
            _ => {}
        }
    }
    found
}

/// The colours of the one text shape whose text starts with `prefix`.
fn colours_of(shapes: &[egui::epaint::TextShape], prefix: &str) -> Vec<egui::Color32> {
    let hits: Vec<&egui::epaint::TextShape> = shapes
        .iter()
        .filter(|t| t.galley.text().starts_with(prefix))
        .collect();
    assert_eq!(hits.len(), 1, "`{prefix}…` must paint once");
    let t = hits[0];
    if let Some(c) = t.override_text_color {
        return vec![c];
    }
    t.galley
        .job
        .sections
        .iter()
        .map(|s| match s.format.color {
            egui::Color32::PLACEHOLDER => t.fallback_color,
            c => c,
        })
        .collect()
}

/// The painted colour of the dock message after typing `line` ⏎.
fn painted_after(line: &str, prefix: &str) -> (Vec<egui::Color32>, Severity) {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, line);
    assert!(
        app.command_feedback.starts_with(prefix),
        "{line}: {}",
        app.command_feedback
    );
    let shapes = text_shapes(&ctx, &mut app);
    (colours_of(&shapes, prefix), app.command_feedback_severity)
}

/// AC 1 — an unknown word is a refusal: `status.warning`.
#[test]
fn ac1_unknown_word_paints_warning() {
    let (colours, severity) = painted_after("hello", "Unknown command");
    assert_eq!(colours, vec![STATUS_WARNING]);
    assert_eq!(severity, Severity::Warning);
}

/// AC 1 — a toggle acknowledgement is info: `text.primary`.
#[test]
fn ac1_toggle_acknowledgement_paints_text_primary() {
    let (colours, severity) = painted_after("grid", "GRID o");
    assert_eq!(colours, vec![TEXT_PRIMARY]);
    assert_eq!(severity, Severity::Info);
}

/// AC 1 — an agent line with no API key is an error: `status.error`.
#[test]
fn ac1_ai_unavailable_paints_error() {
    let (colours, severity) = painted_after(":x", "! AI unavailable");
    assert_eq!(colours, vec![STATUS_ERROR]);
    assert_eq!(severity, Severity::Error);
}

/// AC 1 — a DIST result is info.
#[test]
fn ac1_dist_result_is_info() {
    let (ctx, mut app) = boot();
    for line in ["dist", "0,0", "3,4"] {
        submit_command(&ctx, &mut app, line);
    }
    assert!(app.command_feedback.starts_with("Distance = 5.000"), "{}", app.command_feedback);
    assert_eq!(app.command_feedback_severity, Severity::Info);
    let shapes = text_shapes(&ctx, &mut app);
    assert_eq!(colours_of(&shapes, "Distance ="), vec![TEXT_PRIMARY]);
}

/// AC 1 — `@1,0` with no anchor is a refusal.
#[test]
fn ac1_relative_without_anchor_is_warning() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "@1,0");
    assert_eq!(app.command_feedback, "No base point for relative input.");
    assert_eq!(app.command_feedback_severity, Severity::Warning);
}

/// AC 1 — exporting layers of an unsaved drawing is a refusal.
#[test]
fn ac1_export_layers_unsaved_is_warning() {
    let mut app = App::default();
    app.command_feedback_severity = Severity::Info;
    lasercad::io::action_export_layers(&mut app);
    assert!(app.command_feedback.contains("Save"), "{}", app.command_feedback);
    assert_eq!(app.command_feedback_severity, Severity::Warning);
}
