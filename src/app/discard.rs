//! The discard-confirmation dialog and the window close button (LCV-113,
//! LCV-136) — the egui half of `src/app/file_ops.rs`'s guard state machine.
//!
//! Split out of `file_ops.rs` (ADR 0004 §"The `src/app/mod.rs` seam", the
//! `file_ops.rs` seam) when that file reached the 300-LOC implementation cap
//! and LCV-138 needed to add a line to it. `file_ops.rs` still owns
//! [`PendingAction`], the safe-to-discard signal
//! (`App::has_unsaved_changes` / `App::mark_saved`) and the four
//! `request_*` guards that park an action here; this file only renders the
//! dialog, applies the operator's answer and polls the native close button —
//! the three functions in the old file that took an `&egui::Context`.
//!
//! [`draw_discard_dialog`] renders nothing while
//! `UnsavedGuard::pending_action` is `None`; on a click it delegates to
//! [`apply_dialog_result`], which is the one place that clears the parked
//! action, on both the Discard and the Cancel path. [`poll_close_request`]
//! is the window-X entry point, latched by `UnsavedGuard::exit_confirmed`
//! once a parked `Exit` is confirmed (see its own doc comment for why).

use crate::ui::DialogResult;

use super::{App, PendingAction};

/// The operator's answer to the discard prompt (LCV-169 AC 7).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DiscardChoice {
    /// Save the drawing, then run the parked action if the save landed.
    Save,
    /// Drop the unsaved changes and run the parked action.
    Discard,
    /// Keep the drawing; drop the parked action. The title-bar × too.
    Cancel,
}

/// Render the discard prompt: the message, then `Save`, `Discard` (its text
/// in `palette::DANGER`, LCV-169 AC 6) and `Cancel`, left to right. Returns
/// the operator's answer on the frame it is given; the × answers `Cancel`.
fn discard_window(ctx: &egui::Context) -> Option<DiscardChoice> {
    let mut open = true;
    let mut choice = None;
    egui::Window::new("Discard unsaved changes?")
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("The current drawing has unsaved changes. Continuing will discard them and the undo history.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let danger = egui::RichText::new("Discard").color(crate::render::palette::DANGER);
                if ui.button("Save").clicked() {
                    choice = Some(DiscardChoice::Save);
                }
                if ui.add(egui::Button::new(danger)).clicked() {
                    choice = Some(DiscardChoice::Discard);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(DiscardChoice::Cancel);
                }
            });
        });
    if open {
        choice
    } else {
        Some(DiscardChoice::Cancel)
    }
}

/// Render the discard prompt while an action is parked and apply the answer.
///
/// Called from [`super::panels::draw_dialogs`]. The render and the decision
/// are split on purpose: it is what makes [`apply_dialog_result`] testable
/// directly, with no simulated pointer click.
pub fn draw_discard_dialog(ctx: &egui::Context, app: &mut App) {
    if app.guard.pending_action.is_none() {
        return;
    }
    if let Some(choice) = discard_window(ctx) {
        apply_discard_choice(ctx, app, choice);
    }
}

/// Apply the operator's answer to the parked action (LCV-169 AC 8).
pub fn apply_discard_choice(ctx: &egui::Context, app: &mut App, choice: DiscardChoice) {
    let result = match choice {
        DiscardChoice::Discard => DialogResult::Confirmed,
        DiscardChoice::Save | DiscardChoice::Cancel => DialogResult::Cancelled,
    };
    apply_dialog_result(ctx, app, result);
}

/// Apply the operator's answer to the parked action.
///
/// Takes `UnsavedGuard::pending_action` unconditionally, so both branches
/// clear it — forgetting to clear it on either one would leave a dialog that
/// reopens every frame. On [`DialogResult::Confirmed`] the parked action
/// runs exactly once (`Exit` sets `UnsavedGuard::exit_confirmed` and sends
/// `ViewportCommand::Close`); on [`DialogResult::Cancelled`] nothing else
/// happens and no viewport command is sent.
pub fn apply_dialog_result(ctx: &egui::Context, app: &mut App, result: DialogResult) {
    let action = app.guard.pending_action.take();
    if result != DialogResult::Confirmed {
        return;
    }
    match action {
        Some(PendingAction::New) => app.action_new(),
        Some(PendingAction::Open) => app.action_open(),
        Some(PendingAction::OpenPath(path)) => app.action_open_path(path),
        Some(PendingAction::Exit) => {
            // Latched first (LCV-136) — see `poll_close_request` below.
            app.guard.exit_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        None => {}
    }
}

/// Poll the window close button (the viewport X) once per frame.
///
/// Called from [`App::update_ui`](super::App::update_ui), immediately before
/// [`super::panels::draw_dialogs`]. When
/// `ctx.input(|i| i.viewport().close_requested())` is true this calls
/// [`App::request_exit`]; if that returns `false` (the document is dirty),
/// `ViewportCommand::CancelClose` is sent in the *same* frame — verified
/// against eframe 0.29.1 (`native/epi_integration.rs:284-295`): the
/// integration checks that frame's `viewport_output[ROOT].commands` for
/// `CancelClose` and closes the window otherwise. On a clean document nothing
/// is sent and the window closes normally.
///
/// `UnsavedGuard::exit_confirmed` short-circuits all of the above once set
/// (LCV-136): `ViewportCommand::Close` only *records* a fresh
/// `ViewportEvent::Close` (`egui_winit::process_viewport_command`, verified
/// against the vendored 0.29.1 source) rather than closing the window
/// itself, so `close_requested()` reports `true` again next frame for that
/// same close. `Exit` mutates no document, so `has_unsaved_changes()` is
/// still `true` then — without this guard, `request_exit` would re-park
/// `Exit` and cancel the confirmed close, forever. Once latched, every later
/// close request is let through unconditionally.
pub fn poll_close_request(ctx: &egui::Context, app: &mut App) {
    if !ctx.input(|i| i.viewport().close_requested()) {
        return;
    }
    if app.guard.exit_confirmed {
        return;
    }
    if !app.request_exit() {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::UnsavedGuard;
    use crate::document::CreateLine;
    use crate::geometry::{Line, Vec2};
    use std::path::PathBuf;

    fn some_line() -> Line {
        Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
    }

    fn commit_a_line(app: &mut App) {
        app.history
            .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    }

    /// AC 12 — Confirmed runs the parked action exactly once and clears
    /// `pending_action`, for both a document action (`New`) and `Exit`
    /// (asserted via the returned `FullOutput`'s viewport commands, which is
    /// why this drives `apply_dialog_result` inside a `ctx.run` closure).
    #[test]
    fn confirm_runs_the_parked_action_once() {
        let ctx = egui::Context::default();

        let mut app = App::default();
        commit_a_line(&mut app);
        app.guard.pending_action = Some(PendingAction::New);
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            apply_dialog_result(ctx, &mut app, DialogResult::Confirmed);
        });
        assert!(app.guard.pending_action.is_none());
        assert_eq!(
            app.document.entity_count(),
            0,
            "action_new must have run exactly once"
        );
        assert!(
            !out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Close)
        );

        let mut exit_app = App {
            guard: UnsavedGuard {
                pending_action: Some(PendingAction::Exit),
                ..Default::default()
            },
            ..App::default()
        };
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            apply_dialog_result(ctx, &mut exit_app, DialogResult::Confirmed);
        });
        assert!(exit_app.guard.pending_action.is_none());
        assert!(
            out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Close),
            "a confirmed Exit must send ViewportCommand::Close"
        );
    }

    /// AC 13 — Cancelled clears `pending_action` and leaves every other field
    /// exactly as it was, sending no viewport command even when the parked
    /// action was `Exit`.
    #[test]
    fn cancel_restores_nothing_and_clears_the_pending_action() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        commit_a_line(&mut app);
        app.mark_saved();
        app.current_file = Some(PathBuf::from("keep.svg"));
        app.guard.pending_action = Some(PendingAction::Exit);

        let entity_count = app.document.entity_count();
        let revision = app.history.revision();
        let current_file = app.current_file.clone();
        let saved_revision = app.guard.saved_revision;
        let dirty_since = app.autosave.dirty_since;

        let out = ctx.run(egui::RawInput::default(), |ctx| {
            apply_dialog_result(ctx, &mut app, DialogResult::Cancelled);
        });

        assert!(app.guard.pending_action.is_none());
        assert_eq!(app.document.entity_count(), entity_count);
        assert_eq!(app.history.revision(), revision);
        assert_eq!(app.current_file, current_file);
        assert_eq!(app.guard.saved_revision, saved_revision);
        assert_eq!(app.autosave.dirty_since, dirty_since);
        assert!(
            !out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Close),
            "Cancel must never send a viewport command"
        );
    }

    /// A fresh, empty scratch folder under the system temp dir.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lcv169_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A dirty drawing with one line and a parked `New`.
    fn parked_new() -> App {
        let mut app = App::default();
        commit_a_line(&mut app);
        app.guard.pending_action = Some(PendingAction::New);
        app
    }

    /// LCV-169 AC 8 — Save with a writable current path writes the file,
    /// then runs the parked `New` exactly once.
    #[test]
    fn save_with_a_writable_path_runs_the_parked_action_once() {
        let path = scratch("writable").join("drawing.svg");
        let mut app = parked_new();
        app.current_file = Some(path.clone());
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            apply_discard_choice(ctx, &mut app, DiscardChoice::Save);
        });
        assert!(path.exists(), "the drawing was written");
        assert!(app.guard.pending_action.is_none());
        assert_eq!(app.document.entity_count(), 0, "New ran");
        assert!(app.current_file.is_none(), "New ran exactly once");
    }

    /// LCV-169 AC 8 — Save to a path inside a missing folder fails: the
    /// drawing stays, the parked action is dropped, nothing is parked.
    #[test]
    fn save_that_fails_keeps_the_drawing_and_drops_the_action() {
        let path = scratch("missing").join("no_such_dir").join("drawing.svg");
        let mut app = parked_new();
        app.current_file = Some(path.clone());
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            apply_discard_choice(ctx, &mut app, DiscardChoice::Save);
        });
        assert!(!path.exists());
        assert!(app.guard.pending_action.is_none());
        assert_eq!(app.document.entity_count(), 1, "the drawing stays");
        assert_eq!(app.current_file, Some(path));
    }

    /// LCV-169 AC 8 — Save on an untitled drawing goes to Save As, whose
    /// native dialog is disarmed in tests and panics (ADR 0005) before any
    /// write: the drawing stays and the parked action is already dropped.
    #[test]
    fn save_on_an_untitled_drawing_keeps_it_and_drops_the_action() {
        let mut app = parked_new();
        let ctx = egui::Context::default();
        let reached = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                apply_discard_choice(ctx, &mut app, DiscardChoice::Save);
            });
        }));
        assert!(
            reached.is_err(),
            "control: Save went to the disarmed Save As"
        );
        assert!(app.guard.pending_action.is_none());
        assert_eq!(app.document.entity_count(), 1, "the drawing stays");
        assert!(app.current_file.is_none());
    }
}
