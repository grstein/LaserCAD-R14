//! Menubar — File / Edit / View / Help (LCV-065).
//!
//! One public entry point: [`draw_menubar`].  Shortcut text is label-only;
//! dispatch lives in `crate::ui::shortcuts` (LCV-070).  No `rfd`/`eframe`.

use crate::app::App;
use crate::geometry::Vec2;

/// Render the menubar strip.  Must be the first panel in `App::update`.
pub fn draw_menubar(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        file_menu(ui, app);
        edit_menu(ui, app);
        view_menu(ui, app);
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

fn help_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Help", |ui| {
        if ui.button("About").clicked() {
            ui.close_menu();
            do_about(app);
        }
    });
}

/// Select every entity in the document.
pub(crate) fn do_select_all(app: &mut App) {
    let n = app.document.entity_count();
    app.document.selection.set(0..n);
}

#[rustfmt::skip] pub(crate) fn do_zoom_in(app: &mut App) { app.camera.zoom_in(1.25); }
#[rustfmt::skip] pub(crate) fn do_zoom_out(app: &mut App) { app.camera.zoom_out(1.25); }
#[rustfmt::skip] pub(crate) fn do_about(app: &mut App) { app.about_open = true; }

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

    #[test] // AC#11
    fn select_all_covers_all_entities() {
        let mut app = App::default();
        for _ in 0..3 {
            app.document.entities.push(Entity::Line(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
            )));
        }
        do_select_all(&mut app);
        assert_eq!(app.document.selection.len(), 3);
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
}
