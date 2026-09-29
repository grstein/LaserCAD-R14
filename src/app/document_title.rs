//! Native window title and the autosave-recovery flag (LCV-138).
//!
//! Two small, related pieces of file-identity state, grouped into one struct
//! — the `AgentState` seam (ADR 0004 §"The `src/app/mod.rs` seam") — so
//! `App` gains a single `pub title: DocumentTitleState` field instead of two,
//! keeping `src/app/mod.rs` under its 300-line implementation cap:
//!
//! - [`DocumentTitleState::last_title`] — the title string last sent to the
//!   OS window, compared every frame in [`update_title`] so
//!   `ViewportCommand::Title` is sent only on a change, never once per frame.
//! - [`DocumentTitleState::recovered_from_autosave`] — set once, in
//!   `App::new()`'s recovery branch (`src/app/init.rs`), when the session's
//!   document came back from the crash-safety autosave rather than a normal
//!   load or a blank start; cleared by `App::mark_saved()` once the drawing
//!   is actually saved or replaced (LCV-138 amended AC 4). Read by
//!   `src/ui/statusbar.rs` to show a label distinct from the ordinary
//!   autosave badge.
//!
//! MUST NOT import `eframe` or `rfd`.

use std::path::Path;

use super::App;

/// State this file owns (see the module doc for why the two fields are
/// grouped): the last title string sent to the OS, and whether this
/// session's document was recovered from the crash-safety autosave at boot.
#[derive(Debug, Default)]
pub struct DocumentTitleState {
    /// The title string most recently sent via `ViewportCommand::Title`, or
    /// `None` before the first frame computes one. Compared every frame in
    /// [`update_title`] so the command fires only on a change (AC 2).
    pub last_title: Option<String>,
    /// `true` while this session's document was recovered from the
    /// crash-safety autosave at boot (`App::new()`, LCV-138 AC 4) rather than
    /// loaded normally or started blank. Set in exactly one place, alongside
    /// `app.document = recovered`, and nowhere else; cleared in exactly one
    /// place, [`App::mark_saved`](super::App::mark_saved) (LCV-138 amended
    /// AC 4) — the label's whole point is honest file feedback, so it must
    /// stop naming a state that has stopped being true once the recovered
    /// drawing is actually written to a file, or the operator replaces it
    /// outright via New/Open(/Recent). An autosave flush (`mark_clean()`
    /// alone, with no `mark_saved()`) leaves it set: a crash-safety write is
    /// not the operator saving. `App::default()` — the test constructor —
    /// leaves it `false`.
    pub recovered_from_autosave: bool,
}

impl App {
    /// The native window title (AC 1).
    ///
    /// `"Untitled.svg"` when [`App::current_file`] is `None`, or the file's
    /// name only — via [`Path::file_name`], never the full path, matching
    /// Open Recent's own basename-first display (`src/ui/menubar.rs`) — when
    /// it is `Some`. Prefixed with `"*"` iff [`App::has_unsaved_changes`] is
    /// true; suffixed with `" - "` plus [`crate::APP_TITLE`], so the app name
    /// has one source, not a second copy of the literal.
    pub fn display_title(&self) -> String {
        let name = self
            .current_file
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled.svg");
        let star = if self.has_unsaved_changes() { "*" } else { "" };
        format!("{star}{name} - {}", crate::APP_TITLE)
    }
}

/// Recompute the native title once per frame and push it to the OS only on a
/// change (AC 2).
///
/// Comparing against `app.title.last_title` before sending is what keeps this
/// from becoming a per-frame `ViewportCommand::Title` storm — every frame
/// computes [`App::display_title`], but only a differing result calls
/// `ctx.send_viewport_cmd`. This adds no `ctx.request_repaint*` call: it is a
/// plain comparison against already-live state, not a new wake-up source, so
/// `src/app/viewport/tests.rs`'s `every_repaint_request_in_src_is_conditional`
/// three-site count is unaffected.
pub fn update_title(ctx: &egui::Context, app: &mut App) {
    let title = app.display_title();
    if app.title.last_title.as_deref() != Some(title.as_str()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        app.title.last_title = Some(title);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{CreateLine, Entity};
    use crate::geometry::{Line, Vec2};
    use std::path::PathBuf;

    fn some_line() -> Line {
        Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0))
    }

    /// AC 1 — the four cases spelled out in the demand body, including the
    /// exact literal strings it names.
    #[test]
    fn display_title_covers_the_four_named_cases() {
        let clean_untitled = App::default();
        assert_eq!(clean_untitled.display_title(), "Untitled.svg - LaserCAD v2");

        // No `saved_revision` yet, so a non-empty document alone reports
        // unsaved (`App::has_unsaved_changes`'s `None` branch) — pushing an
        // entity directly is enough, no history commit required.
        let mut dirty_untitled = App::default();
        dirty_untitled
            .document
            .entities
            .push(Entity::Line(some_line()));
        assert_eq!(
            dirty_untitled.display_title(),
            "*Untitled.svg - LaserCAD v2"
        );

        let mut clean_named = App {
            current_file: Some(PathBuf::from("/home/op/drawing.svg")),
            ..App::default()
        };
        clean_named.mark_saved();
        assert_eq!(clean_named.display_title(), "drawing.svg - LaserCAD v2");

        // Named, dirty: a real history.commit after mark_saved moves the
        // revision away from what was saved.
        let mut dirty_named = App {
            current_file: Some(PathBuf::from("/home/op/drawing.svg")),
            ..App::default()
        };
        dirty_named.mark_saved();
        dirty_named.history.commit(
            Box::new(CreateLine::new(some_line())),
            &mut dirty_named.document,
        );
        assert_eq!(dirty_named.display_title(), "*drawing.svg - LaserCAD v2");
    }

    /// AC 1 — the title never carries the directory: only `Path::file_name`,
    /// never `current_file`'s full path.
    #[test]
    fn display_title_never_leaks_the_full_path() {
        let app = App {
            current_file: Some(PathBuf::from("/home/operator/private/job.svg")),
            ..App::default()
        };
        let title = app.display_title();
        assert!(title.contains("job.svg"), "{title:?}");
        assert!(
            !title.contains("private") && !title.contains("operator"),
            "the title must not leak the path's directories: {title:?}"
        );
    }

    /// AC 2 — `update_title` sends nothing on a freshly constructed,
    /// untouched `App` (its title cache is pre-seeded to match,
    /// `App::default`'s own doc comment), sends nothing on a following idle
    /// frame either, and sends exactly one `ViewportCommand::Title` once the
    /// title actually changes. This is the guard against a per-frame command
    /// storm.
    #[test]
    fn update_title_sends_a_command_only_on_a_change() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        assert_eq!(
            app.title.last_title.as_deref(),
            Some("Untitled.svg - LaserCAD v2"),
            "positive control: the constructor must pre-seed the cache"
        );

        let out = ctx.run(egui::RawInput::default(), |ctx| update_title(ctx, &mut app));
        assert!(
            out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .is_empty(),
            "an untouched app's first frame must send nothing: it is already correct"
        );

        // A second, otherwise-idle frame: still nothing changed.
        let out = ctx.run(egui::RawInput::default(), |ctx| update_title(ctx, &mut app));
        assert!(
            out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .is_empty(),
            "an unchanged title must send no command"
        );

        // A real change: current_file set and marked saved.
        app.current_file = Some(PathBuf::from("drawing.svg"));
        app.mark_saved();
        let out = ctx.run(egui::RawInput::default(), |ctx| update_title(ctx, &mut app));
        assert_eq!(
            out.viewport_output[&egui::ViewportId::ROOT].commands,
            vec![egui::ViewportCommand::Title(
                "drawing.svg - LaserCAD v2".to_owned()
            )],
            "a genuine title change must send exactly one command"
        );

        // And another idle frame after that change sends nothing again.
        let out = ctx.run(egui::RawInput::default(), |ctx| update_title(ctx, &mut app));
        assert!(
            out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .is_empty()
        );
    }

    /// AC 2 — this file adds no `ctx.request_repaint*` **call**: the title
    /// update is a plain per-frame comparison, not a new wake-up source, so
    /// `src/app/viewport/tests.rs`'s `every_repaint_request_in_src_is_conditional`
    /// three-site count stays unaffected by this demand. Comment lines are
    /// skipped (mirroring that same test's own `implementation_or_all`
    /// convention) so this file's own doc prose about *not* calling it does
    /// not trip the needle it is proving absent.
    #[test]
    fn this_file_requests_no_repaint() {
        let src = include_str!("document_title.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("document_title.rs must have a test module to bound the scan");
        let needle = concat!("request_", "repaint");
        for line in src[..cfg_test_at].lines() {
            let trimmed = line.trim_start();
            assert!(
                trimmed.starts_with("//") || !trimmed.contains(needle),
                "AC 2: document_title.rs must not add a repaint call site: {trimmed:?}"
            );
        }
    }
}
