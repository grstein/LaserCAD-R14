//! File > Open Recent (LCV-138): the submenu and its display labels. Moved
//! out of `menubar.rs` by LCV-166 T1 with no behaviour change.

use super::row::menu_row;
use crate::app::App;

pub(super) fn recent_submenu(ui: &mut egui::Ui, app: &mut App) {
    // Clone to release the borrow before calling request_open_path.
    let recent: Vec<String> = crate::io::recent_files(&app.settings).to_owned();
    if recent.is_empty() {
        ui.add_enabled_ui(false, |ui| menu_row(ui, None, "No recent files", ""));
        return;
    }
    // Labels disambiguate a colliding basename with one directory of context
    // (LCV-138 AC 5); every entry's tooltip is its full path regardless.
    let labels = recent_labels(&recent);
    for (entry, label) in recent.iter().zip(labels.iter()) {
        if menu_row(ui, None, label, "").on_hover_text(entry).clicked() {
            ui.close();
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
pub(super) fn recent_labels(entries: &[String]) -> Vec<String> {
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
