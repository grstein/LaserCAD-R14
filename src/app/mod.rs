//! Top-level application state and egui wiring.
//!
//! [`App`] owns the live editing state — the [`Document`], the
//! undo/redo [`History`], the [`Camera`] world↔screen transform, and
//! pointer-derived state such as [`last_cursor_world`](App::last_cursor_world).
//! All entity mutation goes through the [`Command`] trait and the history
//! stack; tools never mutate the document directly.
//!
//! The frame body is [`App::update_ui`] (ADR 0002 §A1), an orchestrator that
//! calls one function per phase, each in its own file: `input` (keyboard and
//! text routing), `panels` (chrome, agent panel, dialogs), `viewport` (canvas,
//! pointer, camera), `autosave` (the dirty signal, the debounce and the
//! flush). `ortho`, `snap` and `agent_poll` hold pure helpers those phases
//! call, and `cmdline` resolves one submitted command line into a
//! [`ToolInput`](crate::cmdline::ToolInput) (LCV-111).
//!
//! `mod.rs` re-exports the module's whole public surface, so callers outside
//! `app` use `lasercad::app::…` paths and never a deep one.

mod autosave;
mod cmdline;
mod input;
mod ortho;
mod panels;
mod snap;
mod viewport;

mod agent_poll;
pub use agent_poll::poll_agent_rx;
pub use autosave::autosave_due;
pub use cmdline::submit;
pub use input::process_input;
pub use ortho::apply_ortho;
pub use snap::{resolve_snap, suppress_snap_if_disabled};
pub use viewport::{handle_pan, handle_wheel_zoom, handle_zoom_extents};

use std::path::PathBuf;
use std::time::Instant;

use crate::cmdline::CommandHistory;
use crate::document::{Command, Document, Entity, History};
use crate::geometry::{SnapResult, Vec2};
use crate::io::settings::Settings;
use crate::render::{Bed, Camera};
use crate::tools::ToolManager;

/// Live application state. Owned by the eframe runtime via
/// [`crate::run`] and ticked once per frame in [`App::update_ui`].
pub struct App {
    /// The CAD document — entities, schema, bounds.
    pub document: Document,
    /// Undo/redo history stack (depth = `HISTORY_DEPTH`).
    pub history: History,
    /// World↔screen transform plus zoom and pan state.
    pub camera: Camera,
    /// Laser bed configuration: size and origin in world space.
    pub bed: Bed,
    /// Last known cursor position in world space, updated while hovering
    /// the viewport. `None` before the cursor first enters the panel.
    pub last_cursor_world: Option<Vec2>,
    /// Preview entities: the in-progress tool geometry painted with a
    /// translucent amber stroke. Refreshed from the active tool each frame.
    pub preview_entities: Vec<Entity>,
    /// Active snap result: the snapped point and kind computed by the snap
    /// engine while the cursor hovers the viewport.
    pub active_snap: Option<SnapResult>,
    /// Tool manager: owns the active tool and routes pointer + keyboard events.
    /// Initialized to `SelectTool` by default (LCV-040).
    pub tool_manager: ToolManager,
    /// Persisted user preferences (recent files, etc.). Loaded from the
    /// platform config directory on startup; written back on change (LCV-058).
    pub settings: Settings,
    /// Set to `Some(Instant::now())` the first time the document is dirtied
    /// after the last autosave flush (or after startup). Cleared back to
    /// `None` after each successful autosave write.
    pub dirty_since: Option<Instant>,
    /// The `history.revision()` value last observed by [`App::sync_dirty`] /
    /// [`App::mark_clean`]. Comparing against this — not against
    /// `history.len()`, which is not monotonic across an undo-then-commit —
    /// is how `sync_dirty` detects "something changed since the last
    /// autosave" for every mutation source at once: tools that call
    /// `history.commit` directly, the agent's commit sites, and undo/redo
    /// (ADR 0002 §B).
    pub last_synced_revision: u64,
    /// Controls visibility of the About dialog.
    pub about_open: bool,
    /// Controls visibility of the Agent Settings dialog.
    pub agent_settings_open: bool,
    /// Text buffer for the command-line widget (LCV-068).
    pub command_line_input: String,
    /// The 50-entry command recall ring walked by ArrowUp / ArrowDown while
    /// the command line has focus (LCV-111 AC 20). Every non-empty submitted
    /// line is pushed, rejected input included — recall exists so a typo can
    /// be fixed.
    pub command_history: CommandHistory,
    /// The command line's one-line result message (LCV-111 AC 20). Empty
    /// means "nothing to say". A **display string only**: no control flow
    /// reads it. Cleared at the top of every [`submit`] and by the widget's
    /// Escape branch, so a stale error is always dismissible.
    pub command_feedback: String,
    /// One-shot focus request for the command-line widget (LCV-111 AC 20).
    /// Set by the keyboard gate when an unbound character is typed while the
    /// field is unfocused; consumed with `std::mem::take` by the widget,
    /// which then calls `response.request_focus()`.
    pub focus_command_line: bool,
    /// A plain mirror of the command-line widget's `response.has_focus()`,
    /// rewritten every frame by the widget (LCV-111 AC 20). Read by the
    /// keyboard gate so ArrowUp / ArrowDown recall fires only while the
    /// operator is actually in the field. Writing state from a widget is
    /// allowed; reading a key from one is not (ADR 0003 §E3).
    pub command_line_focused: bool,
    /// Whether snap is active. Toggled by F3 (LCV-070). Defaults to true.
    pub snap_enabled: bool,
    /// Whether the grid is rendered. Toggled by F7 (LCV-070). Defaults to true.
    pub grid_enabled: bool,
    /// Whether ortho mode is active. Toggled by F8 (LCV-070/LCV-053). Defaults to false.
    pub ortho_enabled: bool,
    /// Whether the AI assistant side panel is visible (LCV-080).
    /// Toggled by the 🤖 toolbar button.
    pub agent_panel_open: bool,
    /// Chat history as `(role, content)` pairs (LCV-080).
    /// Role is one of `"user"`, `"assistant"`, or `"error"`.
    pub agent_chat: Vec<(String, String)>,
    /// Live contents of the AI text-input widget; cleared on submit (LCV-080).
    pub agent_input_draft: String,
    /// `true` while a background agent thread is in flight (LCV-080).
    /// The Send button is disabled and a spinner is shown when this is `true`.
    pub agent_busy: bool,
    /// Receiver polled every frame; `Some` while a turn is in flight (LCV-080).
    pub agent_rx: Option<std::sync::mpsc::Receiver<crate::agent::AgentPanelMsg>>,
    /// Path of the file most recently opened or saved. `None` for an unsaved
    /// new document (LCV-062).
    pub current_file: Option<std::path::PathBuf>,
    /// When `Some`, a modal error window is rendered on the next frame; cleared
    /// when the user dismisses it (LCV-062).
    pub error_message: Option<String>,
}

/// The test constructor (ADR 0002 §A2). Touches no filesystem: `settings`
/// is `Settings::default()`, `document` is never replaced with an autosaved
/// or persisted one. Safe to call from any `#[cfg(test)]` context. Boot code
/// must use [`App::new`] instead, which additionally reads the platform
/// config directory and the platform data directory.
impl Default for App {
    fn default() -> Self {
        Self {
            document: Document::default(),
            history: History::default(),
            camera: Camera::default(),
            bed: Bed::default(),
            last_cursor_world: None,
            preview_entities: Vec::new(),
            active_snap: None,
            tool_manager: ToolManager::default(),
            settings: Settings::default(),
            dirty_since: None,
            last_synced_revision: 0,
            about_open: false,
            agent_settings_open: false,
            command_line_input: String::new(),
            command_history: CommandHistory::default(),
            command_feedback: String::new(),
            focus_command_line: false,
            command_line_focused: false,
            snap_enabled: true,
            grid_enabled: true,
            ortho_enabled: false,
            agent_panel_open: false,
            agent_chat: Vec::new(),
            agent_input_draft: String::new(),
            agent_busy: false,
            agent_rx: None,
            current_file: None,
            error_message: None,
        }
    }
}

impl App {
    /// Construct the application: boot-only. Reads the platform **config**
    /// directory (via [`Settings::load`]) and the platform **data**
    /// directory (via [`crate::io::load_autosave`]) — the two real
    /// filesystem locations `App::default()` never touches. MUST NOT be
    /// called from tests (ADR 0002 §A2); tests use [`App::default`].
    ///
    /// Calls [`Self::default()`] for all fields, then:
    /// - loads persisted settings — recent files, agent endpoint, agent API
    ///   key — overwriting the default `settings`;
    /// - overwrites `document` with the autosaved one, if present and
    ///   schema-compatible (LCV-059 AC#1).
    ///
    /// Performs no write of its own: no `settings.save()`, no autosave
    /// write, no file created on the startup path.
    pub fn new() -> Self {
        let mut app = Self {
            settings: Settings::load(),
            ..Self::default()
        };
        if let Some(recovered) = crate::io::load_autosave() {
            app.document = recovered;
        }
        app
    }

    /// Commit a command to the document and history stack.
    ///
    /// A convenience wrapper over `history.commit(cmd, &mut document)` for
    /// callers that already hold `&mut App`. `TextTool` was its last *tool*
    /// caller (LCV-048); LCV-112 moved TEXT's commit to `history.commit`
    /// directly (dropping its `&mut App` requirement, LCV-041 alignment), so
    /// as of that demand **no tool calls this method** — its remaining
    /// callers are the four file actions in `src/io/file_actions.rs` and
    /// test / fixture code across the test suite. Every tool calls
    /// `history.commit` directly with no `&mut App` (LCV-041), and both
    /// paths are covered by the same dirty signal — `App::sync_dirty` reads
    /// `history.revision()`, not this method (LCV-102 / ADR 0002 §B).
    ///
    /// LCV-040 AC#7, AC#8.
    pub fn commit(&mut self, cmd: Box<dyn Command>) {
        self.history.commit(cmd, &mut self.document);
    }

    /// Create a new, empty document (LCV-062).
    pub fn action_new(&mut self) {
        crate::io::action_new(self);
    }

    /// Open a document from disk via a file dialog (LCV-062).
    pub fn action_open(&mut self) {
        crate::io::action_open(self);
    }

    /// Load a document from a known path (LCV-065, used by Open Recent).
    pub fn action_open_path(&mut self, path: PathBuf) {
        crate::io::action_open_path(self, path);
    }

    /// Save the current document to disk (LCV-062).
    pub fn action_save(&mut self) {
        crate::io::action_save(self);
    }

    /// Save the current document to a new path via a save dialog (LCV-065).
    pub fn action_save_as(&mut self) {
        crate::io::action_save_as(self);
    }

    /// The whole frame body (ADR 0002 §A1).
    ///
    /// [`eframe::App::update`] delegates here and does nothing else; headless
    /// regression tests drive this method directly through
    /// [`egui::Context::run`] (`tests/harness/mod.rs`). It is an orchestrator
    /// and must stay one: every phase below lives in its own file, and new
    /// frame work joins one of them rather than this list.
    pub fn update_ui(&mut self, ctx: &egui::Context) {
        // The two — and only two — keyboard readers (ADR 0002 §A6): the
        // shortcut table, then the focus gate. Both run before any panel so
        // Escape cancels the tool in the same frame the command line clears.
        let shortcut_fired = crate::ui::process_shortcuts(ctx, self);
        process_input(ctx, self, shortcut_fired);
        // Clear snap each frame when snap is disabled (LCV-070 AC#16).
        suppress_snap_if_disabled(self.snap_enabled, &mut self.active_snap);

        crate::ui::apply_theme(ctx);
        panels::draw_chrome(ctx, self);

        // Poll agent background thread (LCV-080).
        poll_agent_rx(self);
        if self.agent_busy {
            ctx.request_repaint();
        }
        panels::draw_agent_side_panel(ctx, self);

        viewport::draw(ctx, self);

        // Dirty signal, then the autosave flush (ADR 0002 §B): exactly one
        // `sync_dirty` per frame, unconditional, immediately before the check.
        self.sync_dirty();
        autosave::flush_if_due(self);

        panels::draw_dialogs(ctx, self);
    }
}

impl eframe::App for App {
    /// Delegation only — the frame body lives in [`App::update_ui`] so tests
    /// can drive it without an `eframe::Frame` (ADR 0002 §A1). Nothing else
    /// may be added here: a future demand that needs `&mut eframe::Frame`
    /// passes a narrowed value into `update_ui` instead.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// LCV-070 AC#4 — snap_enabled and grid_enabled default to true.
    #[test]
    fn app_default_snap_enabled_is_true() {
        let app = App::default();
        assert!(app.snap_enabled);
        assert!(app.grid_enabled);
        assert!(!app.ortho_enabled);
    }

    /// LCV-053 AC#1 — `App::default().ortho_enabled` is `false`.
    #[test]
    fn app_default_ortho_is_false() {
        let app = App::default();
        assert!(!app.ortho_enabled);
    }

    /// LCV-062 AC#2 — App::default().current_file is None.
    #[test]
    fn app_default_current_file_is_none() {
        let app = App::default();
        assert!(app.current_file.is_none());
    }

    /// LCV-062 AC#2 — App::default().error_message is None.
    #[test]
    fn app_default_error_message_is_none() {
        let app = App::default();
        assert!(app.error_message.is_none());
    }

    // ── LCV-080 tests ─────────────────────────────────────────────────────────

    /// LCV-080 AC#1 — all five agent fields have the correct default values.
    #[test]
    fn app_default_agent_fields() {
        let a = App::default();
        assert!(!a.agent_panel_open);
        assert!(a.agent_chat.is_empty());
        assert!(!a.agent_busy);
        assert!(a.agent_rx.is_none());
        assert!(a.agent_input_draft.is_empty());
    }

    /// LCV-080 AC#13 — `Reply` message appends an assistant entry and clears
    /// busy + receiver.
    #[test]
    fn agent_rx_reply_updates_chat_and_clears_busy() {
        use crate::agent::AgentPanelMsg;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = App {
            agent_rx: Some(rx),
            agent_busy: true,
            ..App::default()
        };
        tx.send(AgentPanelMsg::Reply("done".into())).unwrap();
        poll_agent_rx(&mut app);
        assert_eq!(
            app.agent_chat.last(),
            Some(&("assistant".into(), "done".into())),
        );
        assert!(!app.agent_busy);
        assert!(app.agent_rx.is_none());
    }

    /// LCV-080 AC#14 — `Error` message appends an error entry and clears
    /// busy + receiver.
    #[test]
    fn agent_rx_error_updates_chat_and_clears_busy() {
        use crate::agent::AgentPanelMsg;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = App {
            agent_rx: Some(rx),
            agent_busy: true,
            ..App::default()
        };
        tx.send(AgentPanelMsg::Error("err".into())).unwrap();
        poll_agent_rx(&mut app);
        assert_eq!(app.agent_chat.last(), Some(&("error".into(), "err".into())));
        assert!(!app.agent_busy);
        assert!(app.agent_rx.is_none());
    }

    // ── LCV-102 tests — autosave dirty tracking (ADR 0002 §B) ──────────────

    use crate::document::CreateLine;
    use crate::geometry::Line;

    fn some_line() -> Line {
        Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
    }

    /// AC 6 — `App::default()` initialises `last_synced_revision` to `0`.
    #[test]
    fn app_default_last_synced_revision_is_zero() {
        let app = App::default();
        assert_eq!(app.last_synced_revision, 0);
    }

    /// AC 9 — the regression test for the defect this demand fixes: tools
    /// bypass `App::commit` and call `history.commit` directly, so
    /// `sync_dirty` must observe that mutation through `History::revision()`
    /// alone. Before the fix, nothing but `App::commit` ever armed
    /// `dirty_since`, so a direct `history.commit` left it `None` forever.
    #[test]
    fn direct_history_commit_marks_document_dirty() {
        let mut app = App::default();
        assert!(app.dirty_since.is_none());

        app.history
            .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
        app.sync_dirty();

        assert!(
            app.dirty_since.is_some(),
            "a history.commit bypassing App::commit must still dirty the document"
        );
    }

    /// AC 7 — `sync_dirty` is idempotent without an intervening mutation, and
    /// preserves the *first* dirty instant rather than refreshing it.
    #[test]
    fn sync_dirty_is_idempotent_without_mutation() {
        let mut app = App::default();
        app.history
            .commit(Box::new(CreateLine::new(some_line())), &mut app.document);

        app.sync_dirty();
        let first = app.dirty_since.expect("first sync must arm dirty_since");

        app.sync_dirty();
        assert_eq!(
            app.dirty_since,
            Some(first),
            "a second sync with no new revision must not move the instant"
        );
    }

    /// AC 8 — `mark_clean` clears `dirty_since` and resyncs the revision, so
    /// an immediately following `sync_dirty` stays clean.
    #[test]
    fn mark_clean_clears_and_resyncs() {
        let mut app = App::default();
        app.history
            .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
        app.sync_dirty();
        assert!(app.dirty_since.is_some());

        app.mark_clean();
        assert!(app.dirty_since.is_none());
        assert_eq!(app.last_synced_revision, app.history.revision());

        app.sync_dirty();
        assert!(
            app.dirty_since.is_none(),
            "no new revision since mark_clean, so sync_dirty must stay clean"
        );
    }

    /// AC 10 — undo re-dirties the document (undone away from what is saved).
    #[test]
    fn undo_marks_document_dirty() {
        let mut app = App::default();
        app.history
            .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
        app.sync_dirty();
        app.mark_clean();
        assert!(app.dirty_since.is_none());

        assert!(app.history.undo(&mut app.document));
        app.sync_dirty();
        assert!(app.dirty_since.is_some(), "undo must dirty the document");
    }

    /// AC 10 — redo re-dirties the document.
    #[test]
    fn redo_marks_document_dirty() {
        let mut app = App::default();
        app.history
            .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
        app.history.undo(&mut app.document);
        app.sync_dirty();
        app.mark_clean();
        assert!(app.dirty_since.is_none());

        assert!(app.history.redo(&mut app.document));
        app.sync_dirty();
        assert!(app.dirty_since.is_some(), "redo must dirty the document");
    }

    /// AC 11 — a no-op undo on a clean, empty history must not dirty it.
    #[test]
    fn no_op_undo_does_not_dirty() {
        let mut app = App::default();
        assert!(!app.history.undo(&mut app.document));
        app.sync_dirty();
        assert!(app.dirty_since.is_none());
    }

    /// AC 12 — replacing `history` with a fresh one and calling `mark_clean`
    /// must not let the very next `sync_dirty` re-dirty the just-loaded
    /// document (this is the case that motivates resyncing the revision
    /// inside `mark_clean`, ADR 0002 §B).
    #[test]
    fn replacing_history_then_mark_clean_stays_clean() {
        let mut app = App::default();
        app.history
            .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
        app.sync_dirty();
        assert!(app.dirty_since.is_some());

        app.history = History::default();
        app.mark_clean();
        assert!(app.dirty_since.is_none());

        app.sync_dirty();
        assert!(
            app.dirty_since.is_none(),
            "a fresh History at revision 0 must not re-dirty after mark_clean"
        );
    }
}
