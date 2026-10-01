//! In-app modal dialogs: confirm, error, and about.
//!
//! All dialogs are pure egui floating windows — no OS dialog, no extra crate.
//! They are stateless helpers; the caller owns any open/visible `bool` flag.
//!
//! The keyboard-shortcuts dialog used to live here too; LCV-134 moved it, whole,
//! to `src/ui/shortcuts_dialog.rs` (ADR 0004 Amended (2)). `src/ui/mod.rs`
//! re-exports it next to these three, so no caller outside `src/ui/` needs to
//! know which file a dialog is in.
//!
//! Every dialog here is **read-only with respect to the keyboard**: none of
//! them reads a key event. `src/ui/shortcuts.rs` and `src/app/input.rs` are
//! the only two key readers in the app (ADR 0002 §A6), and the × button is
//! egui's own [`Window::open`] flag, not a key binding of ours.

use egui::{Align2, Context, Window};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The definitive answer returned by a confirmation dialog.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DialogResult {
    /// The user clicked the confirm button (`confirm_label`, e.g. `"Yes"` or
    /// `"Discard"` — the label is caller-supplied, LCV-113).
    Confirmed,
    /// The user clicked the cancel button (`cancel_label`, e.g. `"No"` or
    /// `"Cancel"` — the label is caller-supplied, LCV-113).
    Cancelled,
}

// ---------------------------------------------------------------------------
// Dialog functions
// ---------------------------------------------------------------------------

/// Render a modal-style confirmation window centered in the viewport.
///
/// The window is not resizable, not collapsible, and has no × close button.
/// The body shows `message` and two buttons, `confirm_label` then
/// `cancel_label`, in that order in a horizontal row (LCV-113: the labels
/// were hard-coded `"Yes"` / `"No"` until the discard-confirmation dialog
/// needed `"Discard"` / `"Cancel"`).
///
/// Returns:
/// - `Some(DialogResult::Confirmed)` on the frame `confirm_label` is clicked.
/// - `Some(DialogResult::Cancelled)` on the frame `cancel_label` is clicked.
/// - `None` every other frame.
///
/// The caller is responsible for holding a `bool` flag and stopping the
/// call once a `Some` result is received.
pub fn confirm_dialog(
    ctx: &Context,
    title: &str,
    message: &str,
    confirm_label: &str,
    cancel_label: &str,
) -> Option<DialogResult> {
    let mut result = None;

    Window::new(title)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(confirm_label).clicked() {
                    result = Some(DialogResult::Confirmed);
                }
                if ui.button(cancel_label).clicked() {
                    result = Some(DialogResult::Cancelled);
                }
            });
        });

    result
}

/// Render a modal-style error window centered in the viewport.
///
/// The window is not resizable and not collapsible. The body shows `message`
/// and a single **Close** button; the title-bar × does the same (LCV-169
/// AC 4).
///
/// Returns `true` on the frame **Close** or × is clicked, `false` every other
/// frame.
///
/// The caller is responsible for holding a `bool` flag and stopping the
/// call once `true` is returned.
pub fn error_dialog(ctx: &Context, title: &str, message: &str) -> bool {
    let mut open = true;
    let mut clicked = false;

    Window::new(title)
        .open(&mut open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            clicked = ui.button("Close").clicked();
        });

    clicked || !open
}

/// Render the About dialog.
///
/// The window is opened/closed via `open`; egui's built-in × button and the
/// **Close** button (LCV-169 AC 4) both set `*open = false`. The body shows
/// the application name, the crate version from `Cargo.toml`, and the license
/// identifier.
///
/// Pass `open: &mut bool` from `App`; the Help → About menu item sets it to
/// `true` (wired by LCV-065).
pub fn about_dialog(ctx: &Context, open: &mut bool) {
    let mut close = false;
    Window::new("About LaserCAD")
        .open(open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("LaserCAD v2");
            ui.label(env!("CARGO_PKG_VERSION"));
            ui.label("MIT OR Apache-2.0");
            ui.add_space(8.0);
            close = ui.button("Close").clicked();
        });
    if close {
        *open = false;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// §1 — confirm_dialog returns None when no button is clicked.
    #[test]
    fn confirm_dialog_returns_none_without_click() {
        let ctx = egui::Context::default();
        let mut captured = None;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            captured = confirm_dialog(ctx, "T", "M", "Yes", "No");
        });
        assert_eq!(captured, None);
    }

    /// LCV-113 AC 10 — the button labels are caller-supplied, not hard-coded
    /// `"Yes"` / `"No"`. Renders with the discard dialog's own labels and
    /// asserts only that nothing panics and no click means no result — the
    /// same shape as the test above, with different labels.
    #[test]
    fn confirm_dialog_renders_custom_labels() {
        let ctx = egui::Context::default();
        let mut captured = None;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            captured = confirm_dialog(
                ctx,
                "Discard unsaved changes?",
                "The current drawing has unsaved changes.",
                "Discard",
                "Cancel",
            );
        });
        assert_eq!(captured, None);
    }

    /// §2 — error_dialog returns false when no button is clicked.
    #[test]
    fn error_dialog_returns_false_without_click() {
        let ctx = egui::Context::default();
        let mut captured = false;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            captured = error_dialog(ctx, "E", "Msg");
        });
        assert!(!captured);
    }

    /// §3 — about_dialog leaves `open` true when no close event is simulated.
    #[test]
    fn about_dialog_leaves_open_true_without_close() {
        let ctx = egui::Context::default();
        let mut open = true;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            about_dialog(ctx, &mut open);
        });
        assert!(open);
    }
}
