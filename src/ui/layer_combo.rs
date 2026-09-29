//! The status-bar layer dropdown (LCV-156 AC 5): shows the current layer and
//! can change it.
//!
//! Returns the picked layer instead of committing, so `draw_statusbar` keeps
//! its shared borrow of `App` for the whole row and commits afterwards.
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::document::LayerId;

/// Width of the closed dropdown, in points: long names are truncated so the
/// status row keeps its LCV-140 budget.
pub(crate) const LAYER_COMBO_WIDTH: f32 = 96.0;

/// Paint the dropdown; `Some(id)` when the operator picked a layer other
/// than the current one this frame.
pub(crate) fn layer_combo(ui: &mut egui::Ui, app: &App) -> Option<LayerId> {
    let doc = &app.document;
    let current = doc.current_layer();
    let name = doc.layer(current).map(|l| l.name.as_str()).unwrap_or("");
    let mut picked = current;
    egui::ComboBox::from_id_salt("status_layer_combo")
        .width(LAYER_COMBO_WIDTH)
        .truncate()
        .selected_text(name)
        .show_ui(ui, |ui| {
            for layer in doc.layers() {
                ui.selectable_value(&mut picked, layer.id, &layer.name);
            }
        })
        .response
        .on_hover_text("Current layer: new entities go here");
    (picked != current).then_some(picked)
}
