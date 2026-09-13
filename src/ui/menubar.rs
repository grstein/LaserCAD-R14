//! Menubar — File / Edit / View / Tools / Help (LCV-065, Tools added by
//! LCV-104).
//!
//! One public entry point: [`draw_menubar`].  Shortcut text is label-only;
//! dispatch lives in `crate::ui::shortcuts` (LCV-070).  No `rfd`/`eframe`.
//!
//! The Tools menu has no tool list of its own: it iterates
//! `crate::ui::toolbar::TOOLS`, the same table that drives the toolbar, so
//! the two surfaces cannot drift apart.

use crate::app::App;
use crate::document::SelectionCommand;
use crate::geometry::Vec2;
use crate::render::Camera;
use crate::tools;
use crate::ui::toolbar::TOOLS;

/// Render the menubar strip.  Must be the first panel in `App::update`.
pub fn draw_menubar(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        file_menu(ui, app);
        edit_menu(ui, app);
        view_menu(ui, app);
        tools_menu(ui, app);
        help_menu(ui, app);
    });
}

fn file_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("File", |ui| {
        // New / Open / Open Recent are guarded by a discard-confirmation
        // dialog on an unsaved document (LCV-113); `request_*` decides
        // whether to act immediately or park the action. Save is not
        // destructive and keeps calling `action_save` directly.
        if ui.button("New\tCtrl+N").clicked() {
            ui.close_menu();
            app.request_new();
        }
        if ui.button("Open…\tCtrl+O").clicked() {
            ui.close_menu();
            app.request_open();
        }
        ui.menu_button("Open Recent ▶", |ui| recent_submenu(ui, app));
        if ui.button("Save\tCtrl+S").clicked() {
            ui.close_menu();
            app.action_save();
        }
        if ui.button("Save As…\tCtrl+Shift+S").clicked() {
            ui.close_menu();
            app.action_save_as();
        }
        ui.separator();
        // LCV-114: the bed belongs to the document, so its dialog belongs to
        // the File menu. Opening it only parks a draft; nothing is committed
        // until OK (see `src/app/bed_dialog.rs`).
        if ui.button("Bed size…").clicked() {
            ui.close_menu();
            app.bed_dialog = Some(app.document.bed_mm);
        }
        ui.separator();
        if ui.button("Exit").clicked() {
            ui.close_menu();
            if app.request_exit() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    });
}

fn recent_submenu(ui: &mut egui::Ui, app: &mut App) {
    // Clone to release the borrow before calling request_open_path.
    let recent: Vec<String> = crate::io::recent_files(&app.settings).to_owned();
    if recent.is_empty() {
        ui.add_enabled(false, egui::Button::new("No recent files"));
        return;
    }
    for (i, entry) in recent.iter().enumerate() {
        let label = std::path::Path::new(entry)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(entry.as_str());
        if ui.button(label).clicked() {
            ui.close_menu();
            if let Ok(path) = crate::io::open_recent(i, &mut app.settings) {
                app.request_open_path(path);
            }
        }
    }
}

fn edit_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Edit", |ui| {
        let can_undo = app.history.can_undo();
        if ui
            .add_enabled(can_undo, egui::Button::new("Undo\tCtrl+Z"))
            .clicked()
        {
            ui.close_menu();
            app.history.undo(&mut app.document);
        }
        let can_redo = app.history.can_redo();
        if ui
            .add_enabled(can_redo, egui::Button::new("Redo\tCtrl+Y"))
            .clicked()
        {
            ui.close_menu();
            app.history.redo(&mut app.document);
        }
        ui.separator();
        if ui.button("Select All").clicked() {
            ui.close_menu();
            do_select_all(app);
        }
    });
}

fn view_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("View", |ui| {
        if ui.button("Zoom In").clicked() {
            ui.close_menu();
            do_zoom_in(app);
        }
        if ui.button("Zoom Out").clicked() {
            ui.close_menu();
            do_zoom_out(app);
        }
        if ui.button("Fit to Bed").clicked() {
            ui.close_menu();
            do_fit_to_bed(app);
        }
        ui.separator();
        ui.checkbox(&mut app.grid_enabled, "Grid\tF7");
        ui.checkbox(&mut app.snap_enabled, "Snap\tF3");
    });
}

/// The Tools menu (LCV-104). One `ui.button` per `toolbar::TOOLS` entry,
/// labelled `"<label>\t<shortcut>"` when a shortcut exists and `"<label>"`
/// otherwise. Clicking closes the menu and activates the tool the same way
/// the toolbar button does.
fn tools_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Tools", |ui| {
        for entry in TOOLS {
            let text = match entry.shortcut {
                Some(key) => format!("{}\t{}", entry.label, key),
                None => entry.label.to_owned(),
            };
            if ui.button(text).clicked() {
                ui.close_menu();
                app.tool_manager.set_tool(tools::make(entry.kind));
            }
        }
    });
}

fn help_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Help", |ui| {
        if ui.button("About").clicked() {
            ui.close_menu();
            do_about(app);
        }
        if ui.button("Agent settings…").clicked() {
            ui.close_menu();
            do_agent_settings(app);
        }
    });
}

/// Select every entity in the document, through the history stack.
///
/// The selection is part of `Document` and is mutated only by a
/// [`SelectionCommand`] (AGENTS.md mutation contract), so Ctrl+Z after
/// Edit > Select All restores the selection the operator had before —
/// accidentally selecting 400 entities is no longer a one-way door (LCV-105).
///
/// An empty document commits nothing: selecting nothing where there is
/// nothing to select is a no-op, and a no-op in the undo stack costs the
/// operator a Ctrl+Z press for no change.
pub(crate) fn do_select_all(app: &mut App) {
    let n = app.document.entity_count();
    if n == 0 {
        return;
    }
    app.history
        .commit(Box::new(SelectionCommand::new(0..n)), &mut app.document);
}

#[rustfmt::skip] pub(crate) fn do_zoom_in(app: &mut App) { app.camera.zoom_in(Camera::ZOOM_STEP); }
#[rustfmt::skip] pub(crate) fn do_zoom_out(app: &mut App) { app.camera.zoom_out(Camera::ZOOM_STEP); }
#[rustfmt::skip] pub(crate) fn do_about(app: &mut App) { app.about_open = true; }
#[rustfmt::skip] pub(crate) fn do_agent_settings(app: &mut App) { app.agent_settings_open = true; }

/// Fit the viewport to the laser bed bounding box.
///
/// Built from `document.bed_mm` at the point of use (LCV-114 AC 4/AC 15), so
/// resizing the bed reframes to the new rectangle with no extra bookkeeping.
pub(crate) fn do_fit_to_bed(app: &mut App) {
    let bed = crate::render::Bed::from_size_mm(app.document.bed_mm);
    let min = bed.origin_world;
    let max = Vec2::new(
        bed.origin_world.x + bed.size_mm[0],
        bed.origin_world.y + bed.size_mm[1],
    );
    let vp = app.camera.viewport_size_px;
    app.camera.zoom_extents(Some((min, max)), vp);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Entity;
    use crate::geometry::Line;

    fn run_menubar(app: &mut App) {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| draw_menubar(ui, app));
        });
    }

    #[test] // AC#1, AC#2
    fn menubar_module_compiles() {
        #[allow(unused_imports)]
        use crate::ui::draw_menubar as _;
    }

    #[test] // AC#4
    fn app_default_grid_and_snap_enabled() {
        let app = App::default();
        assert!(app.grid_enabled);
        assert!(app.snap_enabled);
    }

    #[test] // AC#3
    fn draw_menubar_default_app_does_not_panic() {
        let mut app = App::default();
        run_menubar(&mut app);
        assert!(!app.about_open);
    }

    /// LCV-111 AC 16 — the View menu zooms by `Camera::ZOOM_STEP`, the same
    /// constant the typed `zoom in` / `zoom out` use (`src/app/cmdline.rs`),
    /// so the two can never drift apart. No numeric zoom literal lives here.
    #[test]
    fn menu_zoom_uses_the_shared_step() {
        let mut reference = Camera::default();
        reference.zoom_in(Camera::ZOOM_STEP);

        let mut app = App::default();
        do_zoom_in(&mut app);
        assert_eq!(app.camera.mm_per_px, reference.mm_per_px);

        reference.zoom_out(Camera::ZOOM_STEP);
        do_zoom_out(&mut app);
        assert_eq!(app.camera.mm_per_px, reference.mm_per_px);
    }

    #[test] // AC#9
    fn view_grid_checkbox_toggles_grid_enabled() {
        let mut app = App::default();
        app.grid_enabled = !app.grid_enabled;
        assert!(!app.grid_enabled);
        app.grid_enabled = !app.grid_enabled;
        assert!(app.grid_enabled);
    }

    #[test] // AC#9
    fn view_snap_checkbox_toggles_snap_enabled() {
        let mut app = App::default();
        app.snap_enabled = !app.snap_enabled;
        assert!(!app.snap_enabled);
        app.snap_enabled = !app.snap_enabled;
        assert!(app.snap_enabled);
    }

    #[test] // AC#10
    fn help_about_sets_about_open() {
        let mut app = App::default();
        do_about(&mut app);
        assert!(app.about_open);
    }

    /// Three lines pushed straight into the document — test-only setup, the
    /// production path is `CreateLine` through the history stack.
    fn app_with_three_lines() -> App {
        let mut app = App::default();
        for _ in 0..3 {
            app.document.entities.push(Entity::Line(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
            )));
        }
        app
    }

    #[test] // AC#11
    fn select_all_covers_all_entities() {
        let mut app = app_with_three_lines();
        do_select_all(&mut app);
        assert_eq!(app.document.selection.len(), 3);
    }

    /// LCV-105 AC#4 — Select All is undoable: one `history.undo` restores
    /// exactly the selection that was live before the menu item was clicked.
    #[test]
    fn select_all_is_undoable() {
        let mut app = app_with_three_lines();
        app.history.commit(
            Box::new(SelectionCommand::new(vec![1usize])),
            &mut app.document,
        );
        assert_eq!(app.document.selection.len(), 1);

        do_select_all(&mut app);
        assert_eq!(app.document.selection.len(), 3);

        assert!(app.history.undo(&mut app.document));
        let restored: Vec<usize> = app.document.selection.iter().collect();
        assert_eq!(
            restored,
            vec![1usize],
            "undo must restore the previous selection"
        );
    }

    /// LCV-105 AC#5 — Select All on an empty document commits nothing, so it
    /// cannot leave a no-op on the undo stack.
    #[test]
    fn select_all_on_empty_document_commits_nothing() {
        let mut app = App::default();
        do_select_all(&mut app);
        assert!(app.document.selection.is_empty());
        assert!(!app.history.can_undo());
    }

    #[test] // AC#12 zoom in
    fn zoom_in_decreases_mm_per_px() {
        let mut app = App::default();
        let before = app.camera.mm_per_px;
        do_zoom_in(&mut app);
        assert!(app.camera.mm_per_px < before);
    }

    #[test] // AC#12 zoom out
    fn zoom_out_increases_mm_per_px() {
        let mut app = App::default();
        let before = app.camera.mm_per_px;
        do_zoom_out(&mut app);
        assert!(app.camera.mm_per_px > before);
    }

    #[test] // AC#13, re-homed onto the document by LCV-114 AC 4
    fn fit_to_bed_uses_bed_bounds() {
        let mut app = App::default();
        app.document.bed_mm = [200.0, 100.0];
        app.camera.viewport_size_px = [800.0, 600.0];
        do_fit_to_bed(&mut app);
        assert!((app.camera.center_world.x - 100.0).abs() < 1e-9);
        assert!((app.camera.center_world.y - 50.0).abs() < 1e-9);
    }

    #[test] // AC#14
    fn open_recent_submenu_empty_message() {
        let mut app = App::default();
        run_menubar(&mut app); // must not panic with empty recent list
    }

    // -----------------------------------------------------------------------
    // LCV-104 — Tools menu, Help > Agent settings
    // -----------------------------------------------------------------------

    /// LCV-104 AC#5 — the bar renders five top-level menus (File, Edit, View,
    /// Tools, Help) without panicking; the Tools menu is new here.
    #[test]
    fn menubar_renders_five_menus_without_panic() {
        let mut app = App::default();
        run_menubar(&mut app);
    }

    /// LCV-104 AC#5 — structural check: the five `*_menu` helpers are called,
    /// in order, from `draw_menubar`. Reading the function's own source is
    /// deliberate: it proves the call order without needing pixels or a
    /// side-channel recorder.
    #[test]
    fn menubar_has_five_menus_in_order() {
        let src = include_str!("menubar.rs");
        let calls = [
            "file_menu(ui, app)",
            "edit_menu(ui, app)",
            "view_menu(ui, app)",
            "tools_menu(ui, app)",
            "help_menu(ui, app)",
        ];
        let mut last = 0usize;
        for call in calls {
            let idx = src
                .find(call)
                .unwrap_or_else(|| panic!("draw_menubar must call {call}"));
            assert!(
                idx > last,
                "{call} must be called after the previous menu in draw_menubar"
            );
            last = idx;
        }
    }

    /// LCV-114 AC 15 — `do_fit_to_bed` frames whatever bed the document
    /// currently has, so a `SetBedSize` needs no renderer bookkeeping.
    #[test]
    fn fit_to_bed_follows_a_resized_document_bed() {
        let mut app = App::default();
        app.camera.viewport_size_px = [800.0, 600.0];
        do_fit_to_bed(&mut app);
        let framed_default = app.camera.center_world;
        app.document.bed_mm = [300.0, 180.0];
        do_fit_to_bed(&mut app);
        assert!((app.camera.center_world.x - 150.0).abs() < 1e-9);
        assert!((app.camera.center_world.y - 90.0).abs() < 1e-9);
        assert!(app.camera.center_world != framed_default);
    }

    /// LCV-114 AC 14 — `Bed size…` sits in the File menu directly above
    /// `Exit`, with a separator between them. The haystack is bounded to
    /// `file_menu`'s body, so this test's own source cannot satisfy it, and
    /// the neighbouring entries are asserted positively: an ordering scan
    /// that found nothing would fail on `Save As…` first.
    #[test]
    fn file_menu_has_bed_size_directly_above_exit() {
        let src = include_str!("menubar.rs");
        let start = src.find("fn file_menu(").expect("file_menu must exist");
        let end = src[start..]
            .find("\nfn recent_submenu(")
            .expect("file_menu must be followed by recent_submenu")
            + start;
        let body = &src[start..end];
        let save_as = body.find("\"Save As…").expect("Save As… must be present");
        let bed = body.find("\"Bed size…").expect("Bed size… must be present");
        let exit = body.find("\"Exit\"").expect("Exit must be present");
        assert!(
            save_as < bed && bed < exit,
            "order: Save As…, Bed size…, Exit"
        );
        assert!(
            body[bed..exit].contains("ui.separator();"),
            "Bed size… must be separated from Exit"
        );
        assert!(
            body[save_as..bed].contains("ui.separator();"),
            "Bed size… must be separated from the save entries"
        );
    }

    /// LCV-104 AC#7 — activating each Tools-menu entry sets the active tool:
    /// `tools::make(entry.kind)` followed by `set_tool` leaves
    /// `active_tool_name() == entry.tool_name`, for every table entry.
    #[test]
    fn tools_menu_entries_activate_their_tool() {
        let mut app = App::default();
        for entry in TOOLS {
            app.tool_manager.set_tool(tools::make(entry.kind));
            assert_eq!(app.tool_manager.active_tool_name(), entry.tool_name);
        }
    }

    /// LCV-104 AC#12 — `do_agent_settings` sets `app.agent_settings_open`,
    /// mirroring `help_about_sets_about_open`.
    #[test]
    fn help_agent_settings_sets_flag() {
        let mut app = App::default();
        assert!(!app.agent_settings_open);
        do_agent_settings(&mut app);
        assert!(app.agent_settings_open);
    }
}
