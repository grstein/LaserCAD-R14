//! Menubar — File / Edit / View / Format / Tools / Help (LCV-065, Tools
//! added by LCV-104, Format by LCV-156).
//!
//! One public entry point: [`draw_menubar`].  Shortcut text is label-only;
//! dispatch lives in `crate::ui::shortcuts` (LCV-070).  No `rfd`/`eframe`.
//! Every row is `icon slot | label | shortcut` through `row.rs` (LCV-166).
//!
//! The Tools menu has no tool list of its own: it iterates
//! `crate::ui::toolbar::TOOLS`, the same table that drives the toolbar, so
//! the two surfaces cannot drift apart.

use crate::app::{App, handle_zoom_extents};
use crate::document::SelectionCommand;
use crate::render::Camera;
use crate::tools;
use crate::ui::icons::{menu, modify};
use crate::ui::toolbar::TOOLS;

mod object_snap;
mod recent;
mod row;

use recent::recent_submenu;
use row::{check_row, menu_row, slot_text};

/// Render the menubar strip.  Must be the first panel in `App::update`.
pub fn draw_menubar(ui: &mut egui::Ui, app: &mut App) {
    egui::menu::bar(ui, |ui| {
        file_menu(ui, app);
        edit_menu(ui, app);
        view_menu(ui, app);
        format_menu(ui, app);
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
        if menu_row(ui, Some(menu::new_file), "New", "Ctrl+N").clicked() {
            ui.close_menu();
            app.request_new();
        }
        if menu_row(ui, Some(menu::open), "Open…", "Ctrl+O").clicked() {
            ui.close_menu();
            app.request_open();
        }
        let recent = slot_text(ui, "Open Recent");
        ui.menu_button(recent, |ui| recent_submenu(ui, app));
        if menu_row(ui, Some(menu::save), "Save", "Ctrl+S").clicked() {
            ui.close_menu();
            app.action_save();
        }
        if menu_row(ui, None, "Save As…", "Ctrl+Shift+S").clicked() {
            ui.close_menu();
            app.action_save_as();
        }
        // LCV-156 AC 10: one LaserGRBL file per Output layer, beside the
        // saved drawing; no dialog, the file names go to the feedback line.
        if menu_row(ui, None, "Export Layers", "").clicked() {
            ui.close_menu();
            crate::io::action_export_layers(app);
        }
        ui.separator();
        // LCV-114: the bed belongs to the document, so its dialog belongs to
        // the File menu. Opening it only parks a draft; nothing is committed
        // until OK (see `src/app/bed_dialog.rs`).
        if menu_row(ui, None, "Bed Size…", "").clicked() {
            ui.close_menu();
            app.bed_dialog = Some(app.document.bed_mm);
        }
        ui.separator();
        if menu_row(ui, None, "Exit", "").clicked() {
            ui.close_menu();
            if app.request_exit() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    });
}

fn edit_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Edit", |ui| {
        let can_undo = app.history.can_undo();
        let undo = ui.add_enabled_ui(can_undo, |ui| {
            menu_row(ui, Some(menu::undo), "Undo", "Ctrl+Z")
        });
        if undo.inner.clicked() {
            ui.close_menu();
            app.history.undo(&mut app.document);
        }
        let can_redo = app.history.can_redo();
        let redo = ui.add_enabled_ui(can_redo, |ui| {
            menu_row(ui, Some(menu::redo), "Redo", "Ctrl+Y")
        });
        if redo.inner.clicked() {
            ui.close_menu();
            app.history.redo(&mut app.document);
        }
        // LCV-166: erase the selection, one undo step, as ERASE does.
        let any = !app.document.selection.is_empty();
        let delete = ui.add_enabled_ui(any, |ui| {
            menu_row(ui, Some(modify::delete), "Delete", "Del")
        });
        if delete.inner.clicked() {
            ui.close_menu();
            tools::delete::commit_delete(&mut app.document, &mut app.history);
        }
        ui.separator();
        if menu_row(ui, None, "Select All", "").clicked() {
            ui.close_menu();
            do_select_all(app);
        }
    });
}

fn view_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("View", |ui| {
        if menu_row(ui, Some(menu::zoom_in), "Zoom In", "").clicked() {
            ui.close_menu();
            do_zoom_in(app);
        }
        if menu_row(ui, Some(menu::zoom_out), "Zoom Out", "").clicked() {
            ui.close_menu();
            do_zoom_out(app);
        }
        // LCV-166 AC 5: Zoom Extents (`F`) and Zoom All before Fit to Bed.
        if menu_row(ui, Some(menu::zoom_extents), "Zoom Extents", "F").clicked() {
            ui.close_menu();
            do_zoom_extents(app);
        }
        if menu_row(ui, None, "Zoom All", "").clicked() {
            ui.close_menu();
            do_zoom_all(app);
        }
        if menu_row(ui, Some(menu::fit_bed), "Fit to Bed", "").clicked() {
            ui.close_menu();
            do_fit_to_bed(app);
        }
        ui.separator();
        // The three mode checkboxes flip the same three flags as F7 / F3 / F8
        // and as the status-bar indicators (LCV-116 AC 18): one flag, three
        // paths, no third copy of the state.
        check_row(ui, &mut app.grid_enabled, "Grid", "F7");
        check_row(ui, &mut app.snap_enabled, "Snap", "F3");
        object_snap::object_snap_menu(ui, app); // LCV-161: per-kind toggles
        check_row(ui, &mut app.ortho_enabled, "Ortho", "F8");
    });
}

/// The Tools menu (LCV-104). One row per `toolbar::TOOLS` entry: its rail
/// icon, its label and its key, if any, in the shortcut column (LCV-166).
/// Clicking closes the menu and activates the tool the same way the toolbar
/// button does.
/// R14's Format menu (LCV-156): `Layers…` opens the layer dialog.
fn format_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Format", |ui| {
        if menu_row(ui, None, "Layers…", "").clicked() {
            ui.close_menu();
            app.open_layers_dialog();
        }
    });
}

fn tools_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Tools", |ui| {
        for entry in TOOLS {
            let key = entry.shortcut.unwrap_or_default();
            if menu_row(ui, Some(entry.icon), entry.label, key).clicked() {
                ui.close_menu();
                app.tool_manager.set_tool(tools::make(entry.kind));
            }
        }
    });
}

fn help_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Help", |ui| {
        if menu_row(ui, None, "Keyboard Shortcuts\u{2026}", "F1").clicked() {
            ui.close_menu();
            do_shortcuts(app);
        }
        if menu_row(ui, None, "About", "").clicked() {
            ui.close_menu();
            do_about(app);
        }
        if menu_row(ui, None, "AI Settings…", "").clicked() {
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
#[rustfmt::skip] pub(crate) fn do_shortcuts(app: &mut App) { app.shortcuts_open = true; }
#[rustfmt::skip] pub(crate) fn do_agent_settings(app: &mut App) { app.agent_settings_open = true; }

/// Fit the viewport to the laser bed bounding box.
///
/// Built from `document.bed_mm` at the point of use (LCV-114 AC 4/AC 15), so
/// resizing the bed reframes to the new rectangle with no extra bookkeeping.
pub(crate) fn do_fit_to_bed(app: &mut App) {
    app.camera.frame_bed(app.document.bed_mm);
}

/// Fit the viewport to the drawing extents — the one body behind `F`,
/// `Ctrl+0`, the typed `zoom e` and `View > Zoom Extents` (LCV-166 AC 6).
/// Reads the viewport size the previous frame synced.
pub(crate) fn do_zoom_extents(app: &mut App) {
    let viewport_size = app.camera.viewport_size_px;
    handle_zoom_extents(&mut app.camera, &app.document, viewport_size);
}

/// Frame the bed and the drawing together (`View > Zoom All`, LCV-166 AC 7).
pub(crate) fn do_zoom_all(app: &mut App) {
    app.camera
        .frame_all(app.document.bed_mm, app.document.bounds());
}

#[cfg(test)]
mod tests;
