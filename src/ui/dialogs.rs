//! In-app modal dialogs: confirm, error, and about.
//!
//! All dialogs are pure egui floating windows — no OS dialog, no extra crate.
//! They are stateless helpers; the caller owns any open/visible `bool` flag.

use egui::{Align2, Context, Window};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The definitive answer returned by a confirmation dialog.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DialogResult {
    /// The user clicked **Yes**.
    Confirmed,
    /// The user clicked **No**.
    Cancelled,
}

// ---------------------------------------------------------------------------
// Dialog functions
// ---------------------------------------------------------------------------

/// Render a modal-style confirmation window centered in the viewport.
///
/// The window is not resizable, not collapsible, and has no × close button.
/// The body shows `message` and two buttons: **Yes** and **No**.
///
/// Returns:
/// - `Some(DialogResult::Confirmed)` on the frame **Yes** is clicked.
/// - `Some(DialogResult::Cancelled)` on the frame **No** is clicked.
/// - `None` every other frame.
///
/// The caller is responsible for holding a `bool` flag and stopping the
/// call once a `Some` result is received.
pub fn confirm_dialog(ctx: &Context, title: &str, message: &str) -> Option<DialogResult> {
    let mut result = None;

    Window::new(title)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    result = Some(DialogResult::Confirmed);
                }
                if ui.button("No").clicked() {
                    result = Some(DialogResult::Cancelled);
                }
            });
        });

    result
}

/// Render a modal-style error window centered in the viewport.
///
/// The window is not resizable, not collapsible, and has no × close button.
/// The body shows `message` and a single **OK** button.
///
/// Returns `true` on the frame **OK** is clicked, `false` every other frame.
///
/// The caller is responsible for holding a `bool` flag and stopping the
/// call once `true` is returned.
pub fn error_dialog(ctx: &Context, title: &str, message: &str) -> bool {
    let mut clicked = false;

    Window::new(title)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            if ui.button("OK").clicked() {
                clicked = true;
            }
        });

    clicked
}

/// Render the About dialog.
///
/// The window is opened/closed via `open`; egui's built-in × button sets
/// `*open = false`. The body shows the application name, the crate version
/// from `Cargo.toml`, and the license identifier.
///
/// Pass `open: &mut bool` from `App`; the Help → About menu item sets it to
/// `true` (wired by LCV-065).
pub fn about_dialog(ctx: &Context, open: &mut bool) {
    Window::new("About LaserCAD")
        .open(open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("LaserCAD v2");
            ui.label(env!("CARGO_PKG_VERSION"));
            ui.label("MIT OR Apache-2.0");
        });
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
            captured = confirm_dialog(ctx, "T", "M");
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
