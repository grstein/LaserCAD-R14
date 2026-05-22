//! Top-level application state and egui wiring.
//!
//! [`App`] owns the live editing state — the [`Document`], the
//! undo/redo [`History`], the [`Camera`] world↔screen transform, and
//! pointer-derived state such as [`last_cursor_world`](App::last_cursor_world).
//! All entity mutation goes through `App::commit` once it lands; tools never
//! mutate the document directly.
//!
//! The render pipeline (camera, grid, bed, entities, preview, snaps) attaches
//! to the `CentralPanel` viewport rect set up by [`App::update`].

mod snap;

pub use snap::resolve_snap;

use std::time::{Duration, Instant};

use crate::document::{Command, Document, Entity, History};
use crate::geometry::{SnapResult, Vec2};
use crate::io::settings::Settings;
use crate::render::{Bed, Camera};
use crate::tools::{PointerButton, PointerEvent, ToolManager};

/// Debounce delay before an unsaved change triggers an autosave write.
const AUTOSAVE_DEBOUNCE: Duration = Duration::from_secs(5);

/// Factor applied per mouse-wheel notch. `> 1.0` zooms in; `< 1.0` zooms out.
const WHEEL_ZOOM_FACTOR: f64 = 1.1;

/// Live application state. Owned by the eframe runtime via
/// [`crate::run`] and ticked once per frame in [`App::update`].
#[derive(Default)]
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
    /// Controls visibility of the About dialog.
    pub about_open: bool,
    /// Controls visibility of the Agent Settings dialog.
    /// The LCV-065 menubar will set this to `true`; LCV-076 owns the field.
    pub agent_settings_open: bool,
    /// Text buffer for the command-line widget (LCV-068).
    ///
    /// Bound to the single-line `TextEdit` in the bottom command-line strip.
    /// Submitted (Enter) and cleared to `""` by `draw_command_line`; also
    /// cleared on Escape.
    pub command_line_input: String,
}

impl App {
    /// Construct the application, restoring from autosave if a recovery file
    /// is available.
    ///
    /// Calls [`Self::default()`] for all fields, then overwrites `document`
    /// with the autosaved one (if present and schema-compatible).
    /// LCV-059 AC#1.
    pub fn new() -> Self {
        let mut app = Self::default();
        if let Some(recovered) = crate::io::load_autosave() {
            app.document = recovered;
        }
        app
    }

    /// Commit a command to the document and history stack.
    ///
    /// This is the **only legal path** for tools to mutate the document.
    /// The command is executed via `history.commit(cmd, &mut document)`.
    /// Sets `dirty_since` to the current instant if it is not already set,
    /// starting the autosave debounce timer (LCV-059).
    ///
    /// LCV-040 AC#7, AC#8.
    pub fn commit(&mut self, cmd: Box<dyn Command>) {
        self.history.commit(cmd, &mut self.document);
        self.dirty_since.get_or_insert_with(Instant::now);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::ui::apply_theme(ctx);

        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            crate::ui::draw_statusbar(ui, self);
        });

        egui::TopBottomPanel::bottom("command_line").show(ctx, |ui| {
            crate::ui::draw_command_line(ui, self);
        });

        egui::SidePanel::left("toolbar").show(ctx, |ui| {
            crate::ui::draw_toolbar(ui, self);
        });

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

            crate::render::draw_grid(&painter, rect, &self.camera);
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
                    self.active_snap =
                        resolve_snap(hover_pos, rect, &self.camera, &self.document.entities);
                    let world_pos = self.active_snap.map(|s| s.point).unwrap_or_else(|| {
                        self.camera.screen_to_world(hover_pos - rect.min.to_vec2())
                    });
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

            // Zoom-extents keys.
            if ctx.input(|i| i.key_pressed(egui::Key::F))
                || ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num0))
            {
                handle_zoom_extents(
                    &mut self.camera,
                    &self.document,
                    [rect.width(), rect.height()],
                );
            }

            // Escape key to tool (LCV-040 AC#13).
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                let mut tm = std::mem::take(&mut self.tool_manager);
                tm.handle_key(egui::Key::Escape, self);
                self.tool_manager = tm;
            }

            // Delete / Backspace: route to active tool (LCV-052).
            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
                let mut tm = std::mem::take(&mut self.tool_manager);
                tm.handle_key(egui::Key::Delete, self);
                self.tool_manager = tm;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Backspace)) {
                let mut tm = std::mem::take(&mut self.tool_manager);
                tm.handle_key(egui::Key::Backspace, self);
                self.tool_manager = tm;
            }

            // Character input → active tool (LCV-048 TextTool).
            let typed_chars: Vec<char> = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| {
                        if let egui::Event::Text(t) = e {
                            Some(t.chars().collect::<Vec<char>>())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect()
            });
            for ch in typed_chars {
                self.tool_manager.on_text_input(ch);
            }

            // Autosave flush (LCV-059): fire when the document has been dirty
            // for longer than AUTOSAVE_DEBOUNCE without a flush.
            if let Some(since) = self.dirty_since {
                if since.elapsed() >= AUTOSAVE_DEBOUNCE {
                    let _ = crate::io::save_autosave(&self.document);
                    self.dirty_since = None;
                }
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
    }
}

// ---------------------------------------------------------------------------
// Testable helpers
// ---------------------------------------------------------------------------

/// Apply a wheel-zoom step to `camera` anchored at `screen_anchor`.
pub fn handle_wheel_zoom(camera: &mut Camera, screen_anchor: egui::Pos2, factor: f64) {
    camera.zoom_around(screen_anchor, factor);
}

/// Pan the camera by a screen-space delta (middle-drag).
pub fn handle_pan(camera: &mut Camera, delta_screen_px: egui::Vec2) {
    camera.pan(delta_screen_px);
}

/// Fit the document bounds to the viewport.
pub fn handle_zoom_extents(camera: &mut Camera, document: &Document, viewport_size: [f32; 2]) {
    camera.zoom_extents(document.bounds(), viewport_size);
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
}
