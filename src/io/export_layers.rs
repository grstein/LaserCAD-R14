//! `File > Export layers` (LCV-156 AC 10–12, ADR 0012 §5): one LaserGRBL file
//! per layer, beside the mother file.
//!
//! [`layer_exports`] is the pure plan — which files, with which contents —
//! and [`action_export_layers`] writes it. The live document is exported;
//! the mother file is not saved, and nothing here touches `History`.
//!
//! **Purity rule**: MUST NOT import `egui`, `eframe`, or `rfd`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::app::App;
use crate::document::{Document, file_key};
use crate::io::svg::export_layer_svg;

/// The per-layer files for `doc` saved as `mother`: `<stem>-<file_key>.svg`
/// in `mother`'s folder for every layer with Output on and at least one
/// entity, in layer order, each paired with its SVG text. Distinct layer
/// names have distinct file keys (AC 6), so the paths never collide.
pub fn layer_exports(doc: &Document, mother: &Path) -> Vec<(PathBuf, String)> {
    let stem = mother
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    doc.layers()
        .iter()
        .filter(|layer| layer.output && doc.layer_entity_count(layer.id) > 0)
        .map(|layer| {
            let name = format!("{stem}-{}.svg", file_key(&layer.name));
            (mother.with_file_name(name), export_layer_svg(doc, layer.id))
        })
        .collect()
}

/// Write the [`layer_exports`] plan for the current file, overwriting any
/// existing file, and list the written file names in
/// `app.command_feedback`. An unsaved drawing is asked to be saved first
/// (AC 11); an empty plan writes nothing and says so (AC 12). A write
/// failure stops at that file and sets `app.error_message`.
pub fn action_export_layers(app: &mut App) {
    let Some(mother) = app.current_file.clone() else {
        app.command_feedback = "Save the drawing first, then export its layers.".to_owned();
        return;
    };
    let plan = layer_exports(&app.document, &mother);
    if plan.is_empty() {
        app.command_feedback = "Nothing to export: no layer has Output on and entities.".to_owned();
        return;
    }
    let mut written = Vec::with_capacity(plan.len());
    for (path, svg) in plan {
        if let Err(e) = fs::write(&path, svg.as_bytes()) {
            app.error_message = Some(format!("Could not write '{}': {e}", path.display()));
            return;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        written.push(name.unwrap_or_default());
    }
    app.command_feedback = format!("Exported layers: {}", written.join(", "));
}
