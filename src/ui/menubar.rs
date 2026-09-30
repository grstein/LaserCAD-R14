//! Menubar — File / Edit / View / Format / Tools / Help (LCV-065, Tools
//! added by LCV-104, Format by LCV-156).
//!
//! One public entry point: [`draw_menubar`].  Shortcut text is label-only;
//! dispatch lives in `crate::ui::shortcuts` (LCV-070).  No `rfd`/`eframe`.
//!
//! The Tools menu has no tool list of its own: it iterates
//! `crate::ui::toolbar::TOOLS`, the same table that drives the toolbar, so
//! the two surfaces cannot drift apart.

use crate::app::App;
use crate::document::SelectionCommand;
use crate::render::Camera;
use crate::tools;
use crate::ui::toolbar::TOOLS;

mod object_snap;

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
        // LCV-156 AC 10: one LaserGRBL file per Output layer, beside the
        // saved drawing; no dialog, the file names go to the feedback line.
        if ui.button("Export layers").clicked() {
            ui.close_menu();
            crate::io::action_export_layers(app);
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
    // Labels disambiguate a colliding basename with one directory of context
    // (LCV-138 AC 5); every entry's tooltip is its full path regardless.
    let labels = recent_labels(&recent);
    for (entry, label) in recent.iter().zip(labels.iter()) {
        if ui.button(label).on_hover_text(entry).clicked() {
            ui.close_menu();
            // Deliberately not `crate::io::open_recent`: that function
            // promotes the entry to the front of the list *before* the file
            // is even read, so a missing or malformed file would still
            // reorder — and, on a colliding basename, permanently blend —
            // the list on a failed open (LCV-138 AC 5). `action_open_path`
            // already promotes-to-front and persists on its own success path
            // (`src/io/file_actions.rs`); that is the only outcome that
            // should move this entry at all.
            app.request_open_path(std::path::PathBuf::from(entry));
        }
    }
}

/// Build File > Open Recent's display labels (LCV-138 AC 5): a bare basename
/// when it does not collide with another entry's basename in the same list,
/// or `"parent/name.svg"` — one directory of context plus the basename —
/// when it does. The full path is never shown here; it is every entry's
/// hover text instead (`recent_submenu` above).
fn recent_labels(entries: &[String]) -> Vec<String> {
    let basenames: Vec<&str> = entries.iter().map(|e| basename(e)).collect();
    (0..entries.len())
        .map(|i| {
            let collides = basenames
                .iter()
                .enumerate()
                .any(|(j, b)| j != i && *b == basenames[i]);
            if collides {
                disambiguated(&entries[i])
            } else {
                basenames[i].to_owned()
            }
        })
        .collect()
}

/// The last path component of `entry`, or `entry` itself when it has none.
fn basename(entry: &str) -> &str {
    std::path::Path::new(entry)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(entry)
}

/// `"parent/name.svg"`. Falls back to the basename alone when `entry` has no
/// parent component — not reachable for a real recent-files entry, which is
/// always an absolute path, but keeps this total.
fn disambiguated(entry: &str) -> String {
    let path = std::path::Path::new(entry);
    let name = basename(entry);
    match path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
    {
        Some(parent) => format!("{parent}/{name}"),
        None => name.to_owned(),
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
        // The three mode checkboxes flip the same three flags as F7 / F3 / F8
        // and as the status-bar indicators (LCV-116 AC 18): one flag, three
        // paths, no third copy of the state.
        ui.checkbox(&mut app.grid_enabled, "Grid\tF7");
        ui.checkbox(&mut app.snap_enabled, "Snap\tF3");
        object_snap::object_snap_menu(ui, app); // LCV-161: per-kind toggles
        ui.checkbox(&mut app.ortho_enabled, "Ortho\tF8");
    });
}

/// The Tools menu (LCV-104). One `ui.button` per `toolbar::TOOLS` entry,
/// labelled `"<label>\t<shortcut>"` when a shortcut exists and `"<label>"`
/// otherwise. Clicking closes the menu and activates the tool the same way
/// the toolbar button does.
/// R14's Format menu (LCV-156): `Layers…` opens the layer dialog.
fn format_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Format", |ui| {
        if ui.button("Layers…").clicked() {
            ui.close_menu();
            app.open_layers_dialog();
        }
    });
}

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
        if ui.button("Keyboard shortcuts\u{2026}\tF1").clicked() {
            ui.close_menu();
            do_shortcuts(app);
        }
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
#[rustfmt::skip] pub(crate) fn do_shortcuts(app: &mut App) { app.shortcuts_open = true; }
#[rustfmt::skip] pub(crate) fn do_agent_settings(app: &mut App) { app.agent_settings_open = true; }

/// Fit the viewport to the laser bed bounding box.
///
/// Built from `document.bed_mm` at the point of use (LCV-114 AC 4/AC 15), so
/// resizing the bed reframes to the new rectangle with no extra bookkeeping.
pub(crate) fn do_fit_to_bed(app: &mut App) {
    app.camera.frame_bed(app.document.bed_mm);
}

#[cfg(test)]
mod tests;
