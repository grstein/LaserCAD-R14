//! The `File > Bed size…` modal (LCV-114 AC 14).
//!
//! Two functions, split the same way as the discard dialog (LCV-113 AC 11):
//! [`draw_bed_dialog`] is the egui half — it renders the window and turns a
//! click into a [`DialogResult`] — and [`apply_bed_dialog_result`] is the
//! decision half, callable from a headless test with no pointer.
//!
//! The document's bed is never written here directly: OK commits a
//! [`SetBedSize`] through [`App::commit`], so the change is undoable and
//! dirties the document through the one existing path (LCV-102 / ADR 0002 §B).
//!
//! Reads no keyboard events (ADR 0002 §A6): the two `DragValue`s own their own
//! text-edit state, and `src/app/input.rs` stays the single keyboard gate.
//!
//! MUST NOT import `eframe` or `rfd`.

use super::App;
use crate::document::SetBedSize;
use crate::ui::{DialogKey, DialogResult};
use crate::util::{BED_MAX_MM, BED_MIN_MM, clamp_bed_mm};

/// Render the Bed size… window when `App::bed_dialog` holds a draft.
///
/// A no-op on every frame the dialog is closed. The draft is written back to
/// `App::bed_dialog` each frame so the `DragValue`s keep their edits; nothing
/// reaches the document until OK. A handed-in [`DialogKey`] is a click:
/// Enter is OK and Escape is Cancel (LCV-169 AC 1, AC 3).
///
/// The settings write lives here rather than in [`apply_bed_dialog_result`]
/// for the same reason as the AI Settings window (`src/app/panels.rs`):
/// persisting a preference is a UI-boundary concern, not part of the pure
/// helper that tests call directly. Since LCV-119 that helper could not reach
/// a real user file in any case — [`App::persist_settings`] writes only where
/// `App::new` pointed it, and is a no-op in a test `App` — and it swallows a
/// failed write, as everywhere else.
pub fn draw_bed_dialog(ctx: &egui::Context, app: &mut App, key: Option<DialogKey>) {
    let Some(mut draft) = app.bed_dialog else {
        return;
    };
    let mut result = None;
    let mut open = true;

    egui::Window::new("Bed Size")
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("bed_size_fields").show(ui, |ui| {
                ui.label("Width (mm)");
                ui.add(bed_drag(&mut draft[0]));
                ui.end_row();
                ui.label("Height (mm)");
                ui.add(bed_drag(&mut draft[1]));
                ui.end_row();
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    result = Some(DialogResult::Confirmed);
                }
                if ui.button("Cancel").clicked() {
                    result = Some(DialogResult::Cancelled);
                }
            });
        });

    app.bed_dialog = Some(draft);
    if !open {
        result = result.or(Some(DialogResult::Cancelled)); // × = Cancel (LCV-169 AC 4)
    }
    result = result.or(key.map(|k| match k {
        DialogKey::Enter => DialogResult::Confirmed,
        DialogKey::Escape => DialogResult::Cancelled,
    }));
    if let Some(result) = result
        && apply_bed_dialog_result(app, result)
    {
        app.persist_settings();
    }
}

/// One bed axis as a `DragValue`, held inside the configurable range.
fn bed_drag(v: &mut f64) -> egui::DragValue<'_> {
    egui::DragValue::new(v)
        .range(BED_MIN_MM..=BED_MAX_MM)
        .speed(1.0)
}

/// Apply the operator's answer to the parked draft; returns `true` when the
/// document's bed actually changed.
///
/// Takes `App::bed_dialog` unconditionally, so both branches close the dialog.
/// On [`DialogResult::Confirmed`] each axis goes through [`clamp_bed_mm`] and,
/// **only if the result differs from the current bed**, a [`SetBedSize`] is
/// committed, `Settings::default_bed_mm` is updated in memory and the next
/// frame frames the new bed (LCV-164 AC 7) — an OK that
/// changes nothing must not cost the operator a Ctrl+Z. On
/// [`DialogResult::Cancelled`] nothing else happens at all.
///
/// Persisting the settings is the caller's job — see [`draw_bed_dialog`].
pub fn apply_bed_dialog_result(app: &mut App, result: DialogResult) -> bool {
    let Some(draft) = app.bed_dialog.take() else {
        return false;
    };
    if result != DialogResult::Confirmed {
        return false;
    }
    let bed = [clamp_bed_mm(draft[0]), clamp_bed_mm(draft[1])];
    #[expect(
        clippy::float_cmp,
        reason = "exact: re-confirming the unchanged draft must not commit"
    )]
    let unchanged = bed == app.document.bed_mm;
    if unchanged {
        return false;
    }
    app.commit(Box::new(SetBedSize::new(bed)));
    app.settings.default_bed_mm = bed;
    app.frame_bed_pending = true; // LCV-164 AC 7: frame the new bed
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test `App` with the dialog already open on `draft` — the state the
    /// menu entry produces.
    fn app_with_draft(draft: [f64; 2]) -> App {
        App {
            bed_dialog: Some(draft),
            ..App::default()
        }
    }

    /// LCV-114 AC 14 — OK commits one `SetBedSize`, moves the document's bed
    /// and updates the in-memory settings seed.
    #[test]
    fn ok_commits_a_set_bed_size_and_updates_the_seed() {
        let mut app = App::default();
        let before = app.history.revision();
        app.bed_dialog = Some([128.0, 128.0]);
        assert!(apply_bed_dialog_result(&mut app, DialogResult::Confirmed));
        assert_eq!(app.document.bed_mm, [128.0, 128.0]);
        assert_eq!(app.settings.default_bed_mm, [128.0, 128.0]);
        assert_eq!(app.history.revision(), before + 1);
        assert!(app.history.can_undo());
        assert_eq!(app.bed_dialog, None);
    }

    /// AC 14 — the committed change is undoable like any other edit.
    #[test]
    fn ok_is_undoable() {
        let mut app = app_with_draft([300.0, 180.0]);
        apply_bed_dialog_result(&mut app, DialogResult::Confirmed);
        app.history.undo(&mut app.document);
        assert_eq!(app.document.bed_mm, [400.0, 400.0]);
    }

    /// AC 14 — Cancel changes nothing and still closes the dialog.
    #[test]
    fn cancel_changes_nothing() {
        let mut app = App::default();
        let before = app.history.revision();
        app.bed_dialog = Some([128.0, 128.0]);
        assert!(!apply_bed_dialog_result(&mut app, DialogResult::Cancelled));
        assert_eq!(app.document.bed_mm, [400.0, 400.0]);
        assert_eq!(app.settings.default_bed_mm, [400.0, 400.0]);
        assert_eq!(app.history.revision(), before);
        assert_eq!(app.bed_dialog, None);
    }

    /// AC 14 — the draft is clamped per axis on the way in.
    #[test]
    fn ok_clamps_each_axis() {
        let mut app = app_with_draft([0.0, 5000.0]);
        assert!(apply_bed_dialog_result(&mut app, DialogResult::Confirmed));
        assert_eq!(app.document.bed_mm, [BED_MIN_MM, BED_MAX_MM]);
    }

    /// AC 14 — OK on an unchanged bed commits nothing: no history entry, no
    /// dirty document, nothing to undo.
    #[test]
    fn ok_with_an_unchanged_bed_commits_nothing() {
        let mut app = App::default();
        let before = app.history.revision();
        app.bed_dialog = Some(app.document.bed_mm);
        assert!(!apply_bed_dialog_result(&mut app, DialogResult::Confirmed));
        assert_eq!(app.history.revision(), before);
        assert!(!app.history.can_undo());
    }

    /// A result arriving with no draft parked is a no-op, not a panic.
    #[test]
    fn a_result_without_a_draft_is_a_no_op() {
        let mut app = App::default();
        assert!(!apply_bed_dialog_result(&mut app, DialogResult::Confirmed));
        assert_eq!(app.document.bed_mm, [400.0, 400.0]);
    }

    /// AC 14 — the window is a no-op while closed: `draw_bed_dialog` on a
    /// `None` draft neither panics nor opens anything.
    #[test]
    fn draw_is_a_no_op_while_closed() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            let ctx = &ui.ctx().clone();
            draw_bed_dialog(ctx, &mut app, None)
        });
        assert_eq!(app.bed_dialog, None);
        assert!(!app.history.can_undo());
    }

    /// AC 14 — with a draft parked the window renders and keeps the draft
    /// across frames; no click means no commit.
    #[test]
    fn draw_keeps_the_draft_until_a_button_is_clicked() {
        let ctx = egui::Context::default();
        let mut app = app_with_draft([128.0, 128.0]);
        for _ in 0..2 {
            let _ = ctx.run_ui(Default::default(), |ui| {
                let ctx = &ui.ctx().clone();
                draw_bed_dialog(ctx, &mut app, None)
            });
        }
        assert_eq!(app.bed_dialog, Some([128.0, 128.0]));
        assert_eq!(app.document.bed_mm, [400.0, 400.0]);
        assert!(!app.history.can_undo());
    }
}
