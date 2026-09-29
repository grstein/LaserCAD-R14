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
pub use ortho::apply_ortho;
pub use snap::{resolve_snap, suppress_snap_if_disabled};
pub use unsaved_guard::UnsavedGuard;
pub use viewport::{handle_pan, handle_wheel_zoom, handle_zoom_extents};

use std::time::Instant;

use crate::cmdline::CommandHistory;
use crate::document::{Command, Document, Entity, History};
use crate::geometry::{SnapResult, Vec2};
use crate::io::settings::Settings;
use crate::io::Preset;
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
    /// The LaserGRBL colour group every exported entity is written into
    /// (LCV-115). Read by `action_save` / `action_save_as`, chosen from
    /// `File > Export preset ▸`, shown in the status bar, and adopted from the
    /// file by `action_open` / `action_open_path`.
    ///
    /// **Session state, deliberately not persisted** — not in [`Settings`],
    /// not in the [`Document`], not in the autosave envelope. It resets to
    /// [`Preset::Cut`] at every app start: a preset that survived a restart
    /// would let yesterday's marking job silently cut today's, which burns
    /// through the workpiece. `File > New` does **not** reset it, so three
    /// mark jobs in one session are one choice, not three.
    pub export_preset: Preset,
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
mod tests {
    use super::*;

    /// The roles in `agent.chat`, in order — the shape LCV-123 AC 23 fixes.
    /// Asserting on roles rather than on `last()` is what makes an inserted or
    /// reordered row visible instead of silently shifting the tail.
    fn roles(app: &App) -> Vec<&str> {
        app.agent.chat.iter().map(|(r, _)| r.as_str()).collect()
    }

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

    /// LCV-034 AC#7, re-homed by LCV-114 AC 3/AC 4 — the bed is the
    /// document's, and a blank document starts at the 400 mm default that the
    /// old `App::bed` field used to hold.
    #[test]
    fn app_default_bed_comes_from_the_document() {
        let app = App::default();
        assert_eq!(app.document.bed_mm, [400.0, 400.0]);
        assert_eq!(
            crate::render::Bed::from_size_mm(app.document.bed_mm),
            crate::render::Bed::default()
        );
    }

    /// LCV-114 AC 4 — `App` holds **no** `bed` field: a second copy of the bed
    /// is a second source of truth.
    ///
    /// The haystack is bounded to the implementation section, so this test's
    /// own body cannot satisfy it, and the two positive controls below prove
    /// the scan is looking at real field declarations of exactly the shape it
    /// claims is missing — an absence assertion over a haystack that never
    /// could have matched proves nothing.
    #[test]
    fn app_has_no_bed_field() {
        let src = include_str!("mod.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("mod.rs must have a test module to bound the scan");
        let implementation = &src[..cfg_test_at];
        assert!(
            implementation.contains("pub camera: Camera,"),
            "positive control: the struct's fields must be in the haystack"
        );
        assert!(
            implementation.contains("pub bed_dialog: Option<[f64; 2]>,"),
            "positive control: a field whose name starts with `bed` is present"
        );
        assert!(
            !implementation.contains("pub bed:"),
            "App must not own a bed; the document does (LCV-114 AC 4)"
        );
    }

    /// LCV-119 AC 3 / ADR 0006 — the test constructor is given no real user
    /// location, so every persistence call it can reach is a no-op. This is
    /// the property that stops `cargo test` writing the developer's
    /// `~/.config/lasercad` and deleting their `~/.local/share/lasercad`.
    #[test]
    fn app_default_has_no_persistence_paths() {
        let app = App::default();
        assert_eq!(app.settings_path, None);
        assert_eq!(app.autosave_path, None);
    }

    /// LCV-114 AC 14 — the Bed size… modal starts closed.
    #[test]
    fn app_default_has_no_bed_dialog() {
        assert_eq!(App::default().bed_dialog, None);
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
        assert!(!a.agent.panel_open);
        assert!(a.agent.chat.is_empty());
        assert!(!a.agent.busy);
        assert!(a.agent.rx.is_none());
        assert!(a.agent.input_draft.is_empty());
    }

    /// LCV-080 AC#13, retargeted by LCV-122 AC 4 — `Done` appends an assistant
    /// entry and clears busy + receiver. Same behaviour, new variant name.
    #[test]
    fn agent_rx_done_updates_chat_and_clears_busy() {
        use crate::agent::AgentEvent;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = App {
            agent: AgentState {
                rx: Some(rx),
                busy: true,
                ..AgentState::default()
            },
            ..App::default()
        };
        tx.send(AgentEvent::done("done")).unwrap();
        poll_agent_rx(&mut app);
        assert_eq!(
            app.agent.chat.last(),
            Some(&("assistant".into(), "done".into())),
        );
        assert!(!app.agent.busy);
        assert!(app.agent.rx.is_none());
    }

    /// LCV-080 AC#14, retargeted by LCV-122 AC 4 — `Failed` appends an error
    /// entry and clears busy + receiver.
    #[test]
    fn agent_rx_failed_updates_chat_and_clears_busy() {
        use crate::agent::AgentEvent;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = App {
            agent: AgentState {
                rx: Some(rx),
                busy: true,
                ..AgentState::default()
            },
            ..App::default()
        };
        tx.send(AgentEvent::failed("err")).unwrap();
        poll_agent_rx(&mut app);
        assert_eq!(app.agent.chat.last(), Some(&("error".into(), "err".into())));
        assert!(!app.agent.busy);
        assert!(app.agent.rx.is_none());
    }

    /// ADR 0007 §D11 — a worker that ends without a verdict must still bring
    /// `agent.busy` back down, or `update_ui` requests a repaint every frame
    /// for the rest of the session (the LCV-120 bug, reopened silently).
    ///
    /// No thread and no sleep: dropping the `Sender` is exactly what a
    /// panicking or returning worker does, and `try_recv` reports it
    /// deterministically on the very next call.
    #[test]
    fn a_dropped_sender_ends_the_turn_instead_of_hanging_busy() {
        use crate::agent::AgentEvent;
        let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
        let mut app = App {
            agent: AgentState {
                rx: Some(rx),
                busy: true,
                ..AgentState::default()
            },
            ..App::default()
        };
        drop(tx);
        poll_agent_rx(&mut app);
        assert_eq!(
            app.agent.chat.last(),
            Some(&("error".into(), AGENT_LOST_MESSAGE.to_owned())),
        );
        assert!(!app.agent.busy, "a lost turn must clear agent.busy");
        assert!(app.agent.rx.is_none());
    }

    /// An empty but live channel is a no-op: the turn is still running, so the
    /// receiver must survive to the next frame and `agent.busy` must stay up.
    #[test]
    fn an_empty_channel_keeps_the_turn_alive() {
        use crate::agent::AgentEvent;
        let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
        let mut app = App {
            agent: AgentState {
                rx: Some(rx),
                busy: true,
                ..AgentState::default()
            },
            ..App::default()
        };
        poll_agent_rx(&mut app);
        assert!(app.agent.busy);
        assert!(app.agent.rx.is_some());
        drop(tx);
    }

    /// ADR 0007 §D2 — an `Act` is non-terminal: it mutates the live document
    /// through `Command` + `History`, answers down its own reply channel, and
    /// the same frame goes on to consume the `Done` behind it.
    #[test]
    fn an_act_is_applied_answered_and_followed_by_the_terminal_event() {
        use crate::agent::{AgentAction, AgentEvent, AgentOutcome};
        let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
        let (reply, answers) = std::sync::mpsc::channel::<AgentOutcome>();
        let mut app = App {
            agent: AgentState {
                rx: Some(rx),
                busy: true,
                ..AgentState::default()
            },
            ..App::default()
        };
        // A hand-armed turn opens its group as `arm_turn` would (§D14).
        app.history.begin_group("Agent: test");
        let before = app.history.revision();
        tx.send(AgentEvent::Act {
            action: AgentAction::CreateCircle {
                cx: 1.0,
                cy: 2.0,
                r: 3.0,
            },
            reply,
        })
        .unwrap();
        tx.send(AgentEvent::done("drawn")).unwrap();

        poll_agent_rx(&mut app);

        let outcome = answers.try_recv().expect("the Act must be answered");
        assert!(!outcome.is_refused(), "{outcome:?}");
        assert!(outcome.text().contains("Circle created"), "{outcome:?}");
        assert_eq!(app.document.entity_count(), 1);
        assert_eq!(app.history.revision(), before + 1);
        assert!(app.history.can_undo());
        // LCV-123 AC 23 — the row order of a one-action turn: the action's
        // `tool` row, the terminal row, then the note (AC 22).
        assert_eq!(
            roles(&app),
            ["tool", "assistant", "note"],
            "{:?}",
            app.agent.chat
        );
        assert_eq!(app.agent.chat[0].1, outcome.text(), "verbatim, AC 23");
        assert_eq!(
            app.agent.chat[1],
            ("assistant".to_owned(), "drawn".to_owned())
        );
        assert!(!app.agent.busy);
    }

    /// ADR 0007 §D2, the other half — an `Act` on its own ends **nothing**.
    ///
    /// Its sibling above proves an `Act` is applied and answered, but that
    /// holds just as well for an implementation that answers and then ends the
    /// turn, because the `Done` behind it would tidy up anyway. So here the
    /// `Act` arrives alone: the rendezvous is still open, the worker is still
    /// blocked on the reply it just got, and the next tool call is still to
    /// come. Killing `agent.busy` here would stop the repaints that ADR 0007
    /// §D2 needs to turn the crank, and dropping the receiver would strand the
    /// rest of the turn.
    #[test]
    fn a_lone_act_leaves_the_turn_running() {
        use crate::agent::{AgentAction, AgentEvent, AgentOutcome};
        let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
        let (reply, answers) = std::sync::mpsc::channel::<AgentOutcome>();
        let mut app = App {
            agent: AgentState {
                rx: Some(rx),
                busy: true,
                ..AgentState::default()
            },
            ..App::default()
        };
        // A hand-armed turn opens its group as `arm_turn` would (§D14).
        app.history.begin_group("Agent: test");
        tx.send(AgentEvent::Act {
            action: AgentAction::CreateCircle {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
            },
            reply,
        })
        .unwrap();

        poll_agent_rx(&mut app);

        assert!(answers.try_recv().is_ok(), "the Act must still be answered");
        assert_eq!(app.document.entity_count(), 1);
        assert!(
            app.agent.busy,
            "an Act is not a verdict: the turn is still running"
        );
        assert!(
            app.agent.rx.is_some(),
            "dropping the receiver here strands every later tool call"
        );
        assert_eq!(
            roles(&app),
            ["tool"],
            "an Act writes its transcript row (AC 23) and no verdict"
        );

        // The turn ends only when a terminal event arrives, on a later frame.
        tx.send(AgentEvent::done("drawn")).unwrap();
        poll_agent_rx(&mut app);
        assert_eq!(roles(&app), ["tool", "assistant", "note"]);
        assert_eq!(
            app.agent.chat[1],
            ("assistant".to_owned(), "drawn".to_owned())
        );
        assert!(!app.agent.busy);
        assert!(app.agent.rx.is_none());
    }

    /// The fourth turn exit (ADR 0007 §D11, extended) — the worker vanishes
    /// between sending an `Act` and reading its answer.
    ///
    /// The event channel still looks alive, because the worker's `Sender` for
    /// it has not been dropped yet, so neither `Done`, nor `Failed`, nor
    /// `Disconnected` will ever arrive. Only the dead reply channel says
    /// anything is wrong. Leaving `agent.busy` up here is not a cosmetic bug:
    /// `App::update_ui` requests a repaint on every frame while it is set, so
    /// the app would spin for the rest of the session — LCV-120, reopened, with
    /// a symptom that surfaces nowhere near the agent.
    ///
    /// It writes the same row as a dropped sender: a live worker is blocked on
    /// the other half of this reply channel until the answer arrives, so an
    /// `Err` from `send` means the worker is gone, which means the event
    /// channel is closed too. Falling through would have printed that row one
    /// iteration later — the row is the fact, not the arm.
    ///
    /// Dropping the receiving end before polling is exactly what a panicking
    /// worker does, and `Sender::send` reports it on the next call. No thread,
    /// no sleep.
    #[test]
    fn a_worker_that_stops_listening_mid_act_still_ends_the_turn() {
        use crate::agent::{AgentAction, AgentEvent, AgentOutcome};
        let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
        let (reply, answers) = std::sync::mpsc::channel::<AgentOutcome>();
        let mut app = App {
            agent: AgentState {
                rx: Some(rx),
                busy: true,
                ..AgentState::default()
            },
            ..App::default()
        };
        // A hand-armed turn opens its group as `arm_turn` would (§D14).
        app.history.begin_group("Agent: test");
        tx.send(AgentEvent::Act {
            action: AgentAction::CreateCircle {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
            },
            reply,
        })
        .unwrap();
        drop(answers);

        poll_agent_rx(&mut app);

        assert!(
            !app.agent.busy,
            "a worker that stopped listening must not leave the app spinning"
        );
        assert!(app.agent.rx.is_none(), "nothing more can arrive");
        assert_eq!(
            roles(&app),
            ["tool", "error", "note"],
            "the applied action, the verdict, then the undo shape (AC 22, AC 23)"
        );
        assert_eq!(
            app.agent.chat.get(1),
            Some(&("error".into(), AGENT_LOST_MESSAGE.to_owned())),
            "this exit reports the same fact as a dropped sender — the worker \
             is gone — so it must show the operator the same row (ADR 0007 §D11)"
        );
        assert_eq!(
            app.document.entity_count(),
            1,
            "the action itself was applied before the answer was lost"
        );

        // The event channel is still open — this is the point of the test.
        drop(tx);
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
