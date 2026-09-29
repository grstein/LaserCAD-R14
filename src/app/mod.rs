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
//! flush), `file_ops` (the guarded New / Open / Exit entry points, LCV-113)
//! and `discard` (its egui half: the discard-confirmation dialog and the
//! window close button, split out by ADR 0004 §"The `src/app/mod.rs` seam"),
//! `bed_dialog` (the Bed size… modal, LCV-114), `document_title` (native
//! title + recovery flag, LCV-138), `unsaved_guard` (the three fields behind
//! `App::has_unsaved_changes`, same seam). `init` holds the two `App` constructors — `Default` and
//! [`App::new`] — moved out of this file to stay under the 300-LOC
//! implementation cap (LCV-115); `persist` holds the three methods that are
//! the only readers of the two injected path fields (LCV-119, ADR 0006). `ortho`, `snap` and `agent_poll`
//! hold pure helpers those phases call, and `cmdline` resolves one submitted
//! command line into a [`ToolInput`](crate::cmdline::ToolInput) (LCV-111).
//!
//! Four files carry the agent's UI-side half (ADR 0007 §D8): `agent_poll`
//! drains the thread→UI channel once per frame, `agent_apply` turns one
//! `AgentAction` into one `Command`, `agent_turn` holds [`TurnFence`] and arms
//! a turn whose thread `agent_worker` drives (LCV-142), and
//! `agent_state` holds the plain data those files (and `src/agent/panel.rs`)
//! read and write — [`AgentState`], `App`'s one `agent` field (ADR 0004 §"The
//! `src/app/mod.rs` seam", LCV-136).
//!
//! `mod.rs` re-exports the module's whole public surface, so callers outside
//! `app` use `lasercad::app::…` paths and never a deep one.

mod autosave;
mod bed_dialog;
mod cmdline;
mod discard;
mod document_title;
mod file_ops;
mod init;
mod input;
mod layers;
mod ortho;
mod panels;
mod persist;
mod snap;
mod unsaved_guard;
mod viewport;

mod agent_apply;
mod agent_capture;
mod agent_memory;
mod agent_narrate;
mod agent_poll;
mod agent_state;
mod agent_turn;
mod agent_worker;
pub use agent_apply::apply;
pub use agent_poll::{cancel_turn, poll_agent_rx, AGENT_CANCELLED_MESSAGE, AGENT_LOST_MESSAGE};
pub use agent_state::AgentState;
pub use agent_turn::{arm_turn, config_for, start_turn, TurnFence, TurnState, AGENT_FENCE_REFUSAL};
pub use agent_worker::{run_agent_turn, TurnConfig};
pub use autosave::{autosave_due, schedule_flush_repaint};
pub use bed_dialog::{apply_bed_dialog_result, draw_bed_dialog};
pub(crate) use cmdline::agent_available;
pub use cmdline::submit;
pub use discard::{apply_dialog_result, draw_discard_dialog, poll_close_request};
pub use document_title::DocumentTitleState;
pub use file_ops::PendingAction;
pub use input::process_input;
pub use layers::LayersDialog;
pub use ortho::apply_ortho;
pub use snap::{resolve_snap, suppress_snap_if_disabled};
pub use unsaved_guard::UnsavedGuard;
pub use viewport::{handle_pan, handle_wheel_zoom, handle_zoom_extents};

use std::time::Instant;

use crate::cmdline::CommandHistory;
use crate::document::{Command, Document, Entity, History};
use crate::geometry::{SnapResult, Vec2};
use crate::io::settings::Settings;
use crate::render::Camera;
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
    /// Where `settings` is written back, resolved once at boot (ADR 0006).
    /// `None` means this process does not persist settings and every write is
    /// a no-op — the state `App::default()` leaves it in.
    pub settings_path: Option<std::path::PathBuf>,
    /// Where the crash-recovery autosave file lives, resolved once at boot
    /// (ADR 0006). `None` means this process neither writes nor deletes an
    /// autosave file — the state `App::default()` leaves it in.
    pub autosave_path: Option<std::path::PathBuf>,
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
    /// When the last **successful** autosave write happened this session, or
    /// `None` when none has (LCV-116 AC 7). Written in exactly one place —
    /// `src/app/autosave.rs` — and only on the `Ok` branch, so a failed write
    /// cannot claim a save the operator does not have. Read only by the
    /// status-bar indicator; no control flow depends on it.
    pub last_autosave_at: Option<Instant>,
    /// Controls visibility of the About dialog.
    pub about_open: bool,
    /// Controls visibility of the Keyboard shortcuts dialog (LCV-116 AC 11).
    /// Set by `F1` and by `Help > Keyboard shortcuts…`; cleared by egui's own
    /// × through `Window::open`.
    pub shortcuts_open: bool,
    /// Controls visibility of the Agent Settings dialog.
    pub agent_settings_open: bool,
    /// Draft `[width, height]` of the Bed size… modal, or `None` when it is
    /// closed (LCV-114). The document's bed is only touched when OK is
    /// pressed, through a `SetBedSize` command; see `src/app/bed_dialog.rs`.
    pub bed_dialog: Option<[f64; 2]>,
    /// The open Layers… dialog, `None` when closed (LCV-156, `src/app/layers.rs`).
    pub layers_dialog: Option<LayersDialog>,
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
    /// The agent's UI-side state: chat transcript, panel visibility, the
    /// in-flight turn's channel and fence (ADR 0004 §"The `src/app/mod.rs`
    /// seam", LCV-136). `agent_settings_open` stays here on `App` — it is one
    /// of the dialog-visibility cluster, not a turn's.
    pub agent: AgentState,
    /// Path of the file most recently opened or saved. `None` for an unsaved
    /// new document (LCV-062).
    pub current_file: Option<std::path::PathBuf>,
    /// Native title cache + autosave-recovery flag, grouped the way `agent: AgentState` is (`src/app/document_title.rs`, LCV-138).
    pub title: DocumentTitleState,
    /// When `Some`, a modal error window is rendered on the next frame; cleared
    /// when the user dismisses it (LCV-062).
    pub error_message: Option<String>,
    /// The unsaved-changes guard's state: the last known safe-to-discard
    /// revision, a destructive action parked behind the discard-confirmation
    /// dialog, and whether a parked `Exit` has already been confirmed
    /// (`src/app/unsaved_guard.rs`, ADR 0004 §"The `src/app/mod.rs` seam",
    /// LCV-138). [`App::has_unsaved_changes`] and [`App::mark_saved`]
    /// (`src/app/file_ops.rs`) are its only reader and writer.
    pub guard: UnsavedGuard,
}

impl App {
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
        if self.agent.busy {
            // Load-bearing for *progress*, not just for the spinner (ADR 0007
            // §Consequences). The worker blocks on a reply that only
            // `poll_agent_rx` above can send, and that line only runs inside a
            // frame — so an app that stops painting stalls the turn forever,
            // mid-drawing. No headless test catches its deletion — a test
            // loop drives `update_ui` on its own schedule and never asks egui
            // whether a repaint was requested — so what pins it is a scan of
            // this *condition*: `the_agent_repaint_is_guarded_on_the_busy_flag`
            // in `tests/it/agent/turn.rs`.
            ctx.request_repaint();
        }
        panels::draw_agent_side_panel(ctx, self);

        viewport::draw(ctx, self);

        // Dirty signal, then the autosave flush (ADR 0002 §B): exactly one
        // `sync_dirty` per frame, unconditional, immediately before the check.
        // `schedule_flush_repaint` runs *after* the flush, so a write that just
        // happened schedules nothing; only a still-pending one does (LCV-116
        // AC 9 — never an unconditional per-frame repaint).
        self.sync_dirty();
        autosave::flush_if_due(self);
        autosave::schedule_flush_repaint(ctx, self);

        // Window close button (LCV-113): cancel the close and park
        // `PendingAction::Exit` when the document is dirty.
        poll_close_request(ctx, self);
        panels::draw_dialogs(ctx, self);
        // Native window title (LCV-138), last, so it reflects this frame's own changes above; sends nothing unless the title actually changed (AC 2, no new repaint call site).
        document_title::update_title(ctx, self);
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
mod tests;
