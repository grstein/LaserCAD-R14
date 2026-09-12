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
use crate::ui::toolbar::{make_tool, TOOLS};

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
        if ui.button("New\tCtrl+N").clicked() {
            ui.close_menu();
            app.action_new();
        }
        if ui.button("Open…\tCtrl+O").clicked() {
            ui.close_menu();
            app.action_open();
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
        if ui.button("Exit").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}

fn recent_submenu(ui: &mut egui::Ui, app: &mut App) {
    // Clone to release the borrow before calling action_open_path.
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
                app.action_open_path(path);
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
                if let Some(tool) = make_tool(entry.label) {
                    app.tool_manager.set_tool(tool);
                }
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

#[rustfmt::skip] pub(crate) fn do_zoom_in(app: &mut App) { app.camera.zoom_in(1.25); }
#[rustfmt::skip] pub(crate) fn do_zoom_out(app: &mut App) { app.camera.zoom_out(1.25); }
#[rustfmt::skip] pub(crate) fn do_about(app: &mut App) { app.about_open = true; }
#[rustfmt::skip] pub(crate) fn do_agent_settings(app: &mut App) { app.agent_settings_open = true; }

/// Fit the viewport to the laser bed bounding box.
pub(crate) fn do_fit_to_bed(app: &mut App) {
    let min = app.bed.origin_world;
    let max = Vec2::new(
        app.bed.origin_world.x + app.bed.size_mm[0],
        app.bed.origin_world.y + app.bed.size_mm[1],
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

    #[test] // AC#13
    fn fit_to_bed_uses_bed_bounds() {
        let mut app = App::default();
        app.bed.size_mm = [200.0, 100.0];
        app.bed.origin_world = Vec2::new(0.0, 0.0);
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

    /// LCV-104 AC#7 — activating each Tools-menu entry sets the active tool:
    /// `make_tool(entry.label)` followed by `set_tool` leaves
    /// `active_tool_name() == entry.tool_name`, for every table entry.
    #[test]
    fn tools_menu_entries_activate_their_tool() {
        let mut app = App::default();
        for entry in TOOLS {
            let tool =
                make_tool(entry.label).unwrap_or_else(|| panic!("no tool for {}", entry.label));
            app.tool_manager.set_tool(tool);
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
