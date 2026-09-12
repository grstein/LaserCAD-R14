//! Top-level application state and egui wiring.
//!
//! [`App`] owns the live editing state — the [`Document`], the
//! undo/redo [`History`], the [`Camera`] world↔screen transform, and
//! pointer-derived state such as [`last_cursor_world`](App::last_cursor_world).
//! All entity mutation goes through `App::commit` once it lands; tools never
//! mutate the document directly.
//!
//! The render pipeline (camera, grid, bed, entities, preview, snaps) attaches
//! to the `CentralPanel` viewport rect set up by [`App::update_ui`], the frame
//! body that `eframe::App::update` delegates to (ADR 0002 §A1).

mod input;
mod ortho;
mod snap;

mod agent_poll;
pub use agent_poll::poll_agent_rx;
pub use input::process_input;
pub use ortho::apply_ortho;
pub use snap::resolve_snap;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::document::{Command, Document, Entity, History};
use crate::geometry::{SnapResult, Vec2};
use crate::io::settings::Settings;
use crate::render::{Bed, Camera};
use crate::tools::{PointerButton, PointerEvent, ToolManager};

/// Debounce delay before an unsaved change triggers an autosave write.
/// 800 ms matches LaserCAD v1 (ADR 0002 §B).
const AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(800);

/// Factor applied per mouse-wheel notch. `> 1.0` zooms in; `< 1.0` zooms out.
const WHEEL_ZOOM_FACTOR: f64 = 1.1;

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
    /// translucent amber stroke. Defaults to empty; the Phase-4 Tool trait
    /// (LCV-040) will populate this each frame.
    pub preview_entities: Vec<Entity>,
    /// Active snap result: the snapped point and kind computed by the snap
    /// engine. Defaults to `None`; LCV-054 (tool snap integration) writes it.
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
    /// `TextTool` is the only caller of this method; every other tool calls
    /// `history.commit` directly with no `&mut App` (LCV-041). Both paths are
    /// covered by the same dirty signal: [`App::sync_dirty`] reads
    /// `history.revision()`, not this method (LCV-102 / ADR 0002 §B).
    ///
    /// LCV-040 AC#7, AC#8.
    pub fn commit(&mut self, cmd: Box<dyn Command>) {
        self.history.commit(cmd, &mut self.document);
    }

    /// The **only** writer of `dirty_since = Some(_)` (ADR 0002 §B). Called
    /// exactly once per frame, at the end of [`App::update_ui`], immediately
    /// before the autosave-flush check.
    ///
    /// Compares `history.revision()` against `last_synced_revision`: if they
    /// differ, something committed, undid, or redid since the last sync, so
    /// `dirty_since` is armed via `get_or_insert_with(Instant::now)` — which
    /// preserves an already-set instant, so the debounce is measured from the
    /// *first* unsaved change, not the latest one — and `last_synced_revision`
    /// is advanced to the current revision. Calling it twice with no
    /// intervening mutation is a no-op the second time.
    fn sync_dirty(&mut self) {
        let revision = self.history.revision();
        if revision != self.last_synced_revision {
            self.dirty_since.get_or_insert_with(Instant::now);
            self.last_synced_revision = revision;
        }
    }

    /// The **only** writer that resets `dirty_since` to `None`. Clears the
    /// debounce timer and resyncs `last_synced_revision` to the current
    /// `history.revision()` in one step (ADR 0002 §B).
    ///
    /// Resyncing the revision here — not just clearing `dirty_since` — is
    /// mandatory wherever `history` is replaced with a fresh one (`action_new`
    /// / `action_open` / `action_open_path`): a fresh `History` reports
    /// revision `0`, and without resyncing, the very next frame's
    /// `sync_dirty` would see `0 != last_synced_revision` and re-dirty a
    /// document that was just loaded or reset. Call this **after** any
    /// `history` replacement, never before.
    pub fn mark_clean(&mut self) {
        self.dirty_since = None;
        self.last_synced_revision = self.history.revision();
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
    /// [`egui::Context::run`] (`tests/harness/mod.rs`). Keep it an
    /// orchestrator: each phase belongs in its own function so LCV-105 can
    /// split `src/app.rs` mechanically.
    pub fn update_ui(&mut self, ctx: &egui::Context) {
        // The two — and only two — keyboard readers (ADR 0002 §A6): the
        // shortcut table, then the focus gate. Both run before any panel so
        // Escape cancels the tool in the same frame the command line clears.
        crate::ui::process_shortcuts(ctx, self);
        process_input(ctx, self);
        // Clear snap each frame when snap is disabled (LCV-070 AC#16).
        suppress_snap_if_disabled(self.snap_enabled, &mut self.active_snap);

        crate::ui::apply_theme(ctx);

        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            crate::ui::draw_menubar(ui, self);
        });

        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            crate::ui::draw_statusbar(ui, self);
        });

        egui::TopBottomPanel::bottom("command_line").show(ctx, |ui| {
            crate::ui::draw_command_line(ui, self);
        });

        egui::SidePanel::left("toolbar").show(ctx, |ui| {
            crate::ui::draw_toolbar(ui, self);
        });

        // Poll agent background thread (LCV-080).
        poll_agent_rx(self);
        if self.agent_busy {
            ctx.request_repaint();
        }

        // Agent panel (LCV-080) — rendered only when open.
        if self.agent_panel_open {
            egui::SidePanel::right("agent_panel")
                .resizable(true)
                .default_width(300.0)
                .show(ctx, |ui| {
                    crate::agent::draw_agent_panel(ui, self);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

            // Sync the camera's viewport size before any draw call consumes it.
            self.camera.viewport_size_px = [rect.width(), rect.height()];

            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, crate::ui::CANVAS_BG);
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(64)),
            );

            if self.grid_enabled {
                crate::render::draw_grid(&painter, rect, &self.camera);
            }
            crate::render::draw_bed(&painter, rect, &self.camera, &self.bed);
            crate::render::draw_entities(
                &painter,
                rect,
                &self.camera,
                &self.document.entities,
                crate::render::PaintOptions::default(),
            );
            crate::render::draw_selection_highlight(
                &painter,
                rect,
                &self.camera,
                &self.document.entities,
                &self.document.selection,
            );

            // Update preview from tool (LCV-040 AC#9).
            self.preview_entities = self.tool_manager.preview();

            crate::render::draw_preview(&painter, rect, &self.camera, &self.preview_entities);

            if let Some(snap) = &self.active_snap {
                crate::render::draw_snap_marker(&painter, rect, &self.camera, snap);
            }

            // --- pointer / camera interaction (LCV-032 / LCV-041) ---
            if response.hovered() {
                if let Some(hover_pos) = response.hover_pos() {
                    // Resolve snap; viewport-local pos = hover_pos - rect.min.
                    if self.snap_enabled {
                        self.active_snap =
                            resolve_snap(hover_pos, rect, &self.camera, &self.document.entities);
                    }
                    let world_pos = self.active_snap.map(|s| s.point).unwrap_or_else(|| {
                        self.camera.screen_to_world(hover_pos - rect.min.to_vec2())
                    });
                    // Ortho lock (LCV-053): clamp to nearest cardinal axis from
                    // the active tool's anchor, when ortho mode is active.
                    let world_pos = if self.ortho_enabled {
                        match self.tool_manager.anchor() {
                            Some(anchor) => apply_ortho(anchor, world_pos),
                            None => world_pos,
                        }
                    } else {
                        world_pos
                    };
                    self.last_cursor_world = Some(world_pos);

                    // Pointer events (LCV-041).
                    if ctx.input(|i| i.pointer.primary_pressed()) {
                        let shift = ctx.input(|i| i.modifiers.shift);
                        self.tool_manager.on_pointer_event(
                            &PointerEvent::Press {
                                world_pos,
                                button: PointerButton::Primary,
                                shift,
                            },
                            &mut self.document,
                            &mut self.history,
                        );
                        // LCV-049: poll for a successor tool after press events.
                        if let Some(t) = self.tool_manager.take_successor() {
                            self.tool_manager.set_tool(t);
                        }
                    }
                    self.tool_manager.on_pointer_event(
                        &PointerEvent::Move { world_pos },
                        &mut self.document,
                        &mut self.history,
                    );
                    if ctx.input(|i| i.pointer.primary_released()) {
                        let shift = ctx.input(|i| i.modifiers.shift);
                        self.tool_manager.on_pointer_event(
                            &PointerEvent::Release {
                                world_pos,
                                button: PointerButton::Primary,
                                shift,
                            },
                            &mut self.document,
                            &mut self.history,
                        );
                        // LCV-049: poll for a successor tool after release events.
                        if let Some(t) = self.tool_manager.take_successor() {
                            self.tool_manager.set_tool(t);
                        }
                    }

                    // Wheel zoom around cursor.
                    let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
                    if scroll_y != 0.0 {
                        let factor = if scroll_y > 0.0 {
                            WHEEL_ZOOM_FACTOR
                        } else {
                            1.0 / WHEEL_ZOOM_FACTOR
                        };
                        handle_wheel_zoom(&mut self.camera, hover_pos, factor);
                    }
                }
            }

            // Middle-button pan.
            if response.dragged_by(egui::PointerButton::Middle) {
                handle_pan(&mut self.camera, response.drag_delta());
            }

            // No key is read here: `src/app/input.rs` is the single gate
            // (LCV-103 / ADR 0002 §A6).

            // Sync the dirty signal from the history revision (ADR 0002 §B).
            // Exactly one call per frame, unconditional, right before the
            // flush check below.
            self.sync_dirty();

            // Autosave flush (LCV-059): fire when the document has been dirty
            // for longer than AUTOSAVE_DEBOUNCE without a flush.
            if autosave_due(self.dirty_since, Instant::now()) {
                let _ = crate::io::save_autosave(&self.document);
                // Cleared unconditionally: a failed write is dropped, not
                // retried every frame. The next document change re-arms the
                // debounce (AC 18).
                self.mark_clean();
            }

            // Always repaint so cursor-coords and smooth camera motion stay live.
            ctx.request_repaint();
        });

        // Modal dialogs (LCV-069) — rendered after the CentralPanel so they
        // float above the canvas.
        crate::ui::about_dialog(ctx, &mut self.about_open);

        // LCV-076 — Agent Settings dialog.
        {
            let open = &mut self.agent_settings_open;
            let settings = &mut self.settings;
            let was_open = *open;
            egui::Window::new("Agent Settings")
                .open(open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    crate::agent::settings_ui::draw_agent_settings(ui, settings);
                });
            // Save on dialog close (× button or programmatic close).
            if was_open && !*open {
                settings.save().ok();
            }
        }

        // LCV-062 — Error modal: rendered last so it floats above everything.
        if let Some(msg) = self.error_message.clone() {
            if crate::ui::error_dialog(ctx, "Error", &msg) {
                self.error_message = None;
            }
        }
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

// ---------------------------------------------------------------------------
// Testable helpers
// ---------------------------------------------------------------------------

/// Is an armed dirty timer old enough to flush? `dirty_since` is `None` when
/// the document is clean; `Some(t)` means it has been unsaved-dirty since
/// `t`. `now` is a parameter (not `Instant::now()` internally) so the
/// boundary can be tested without a real clock delay (ADR 0002 §A4 rule 2). The
/// boundary is inclusive: exactly `AUTOSAVE_DEBOUNCE` elapsed is due.
pub fn autosave_due(dirty_since: Option<Instant>, now: Instant) -> bool {
    match dirty_since {
        Some(since) => now.saturating_duration_since(since) >= AUTOSAVE_DEBOUNCE,
        None => false,
    }
}

/// Apply a wheel-zoom step to `camera` anchored at `screen_anchor`.
pub fn handle_wheel_zoom(camera: &mut Camera, screen_anchor: egui::Pos2, factor: f64) {
    camera.zoom_around(screen_anchor, factor);
}

/// Pan the camera by a screen-space delta (middle-drag).
pub fn handle_pan(camera: &mut Camera, delta_screen_px: egui::Vec2) {
    camera.pan(delta_screen_px);
}

/// Fit the document bounds to the viewport.
///
/// No-ops on a zero-area viewport (LCV-103). `Camera::default()` carries the
/// `[0.0, 0.0]` sentinel until the first `CentralPanel` syncs a real size, and
/// the keyboard gate runs before that panel — zooming to a zero-area rect
/// would reset the camera for no reason.
pub fn handle_zoom_extents(camera: &mut Camera, document: &Document, viewport_size: [f32; 2]) {
    if viewport_size[0] <= 0.0 || viewport_size[1] <= 0.0 {
        return;
    }
    camera.zoom_extents(document.bounds(), viewport_size);
}

/// Clear `active_snap` when snap is disabled.
///
/// Called at the top of each frame (before panel rendering) in `App::update_ui`
/// so that no snap marker is rendered while snap is turned off, even if
/// `active_snap` was set by a previous frame. Extracted here for testability
/// (LCV-070 AC#16).
pub fn suppress_snap_if_disabled(snap_enabled: bool, active_snap: &mut Option<SnapResult>) {
    if !snap_enabled {
        *active_snap = None;
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
        let doc = Document::default();
        handle_zoom_extents(&mut cam, &doc, [800.0, 600.0]);
        assert_eq!(cam.center_world, Vec2::new(0.0, 0.0));
        assert_eq!(cam.mm_per_px, 1.0);
        assert_eq!(cam.viewport_size_px, [800.0, 600.0]);
    }

    /// LCV-103 AC#14 — `handle_zoom_extents` no-ops on a zero-area viewport.
    ///
    /// The keyboard gate runs before the `CentralPanel`, so on frame 0 it
    /// reads the `[0.0, 0.0]` sentinel of a fresh `Camera`. Pressing `F`
    /// then must leave the camera exactly as it was.
    #[test]
    fn zoom_extents_noop_on_zero_area_viewport() {
        let mut cam = Camera {
            center_world: Vec2::new(7.0, -3.0),
            mm_per_px: 4.0,
            viewport_size_px: [0.0, 0.0],
        };
        let doc = Document::default();
        handle_zoom_extents(&mut cam, &doc, [0.0, 0.0]);
        assert_eq!(cam.center_world, Vec2::new(7.0, -3.0));
        assert_eq!(cam.mm_per_px, 4.0);
        assert_eq!(cam.viewport_size_px, [0.0, 0.0]);

        // A viewport with one zero dimension is equally degenerate.
        handle_zoom_extents(&mut cam, &doc, [800.0, 0.0]);
        assert_eq!(cam.mm_per_px, 4.0);
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

    /// LCV-070 AC#16 — suppress_snap_if_disabled clears active_snap when disabled.
    #[test]
    fn suppress_snap_clears_when_disabled() {
        use crate::geometry::{SnapKind, SnapResult};
        let snap = Some(SnapResult {
            point: Vec2::new(1.0, 2.0),
            kind: SnapKind::Endpoint,
            primary_idx: 0,
            secondary_idx: None,
        });

        let mut active = snap;
        suppress_snap_if_disabled(false, &mut active);
        assert!(active.is_none());

        let mut active2 = snap;
        suppress_snap_if_disabled(true, &mut active2);
        assert!(active2.is_some());
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

    /// AC 15 — the debounce constant matches v1 (800 ms).
    #[test]
    fn autosave_debounce_is_800ms() {
        assert_eq!(AUTOSAVE_DEBOUNCE, Duration::from_millis(800));
    }

    /// AC 16 — `autosave_due` boundary cases, none of which pauses the clock.
    #[test]
    fn autosave_due_boundaries() {
        let now = Instant::now();
        assert!(!autosave_due(None, now), "clean document is never due");
        assert!(!autosave_due(
            now.checked_sub(Duration::from_millis(799)),
            now
        ));
        assert!(
            autosave_due(now.checked_sub(Duration::from_millis(800)), now),
            "boundary is inclusive (>=)"
        );
        assert!(autosave_due(now.checked_sub(Duration::from_secs(5)), now));
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
