//! The `Format > Layers…` window (LCV-156 AC 4).
//!
//! The egui half only: it paints the layer list and the selected layer's
//! edit buffers, and turns a click into one of the `App::layers_*` calls in
//! `src/app/layers.rs`, which own every decision and every commit. The list
//! scrolls inside [`LIST_MAX_HEIGHT`] so the body stays under the ADR 0009
//! cap however many layers there are.
//!
//! Reads no keyboard events (ADR 0002 §A6); the name field owns its own
//! text-edit state. MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::document::LayerId;

/// Height of the scrolling layer list, in points (ADR 0009: the whole body
/// must fit in 426pt).
pub(crate) const LIST_MAX_HEIGHT: f32 = 160.0;

/// Side of a layer's color swatch, in points.
const SWATCH_SIZE: f32 = 12.0;

/// What the operator clicked this frame; applied after the window closes
/// its borrows.
enum Click {
    Select(LayerId),
    New,
    Apply,
    Delete,
    SetCurrent,
    MoveSelection,
    Close,
}

/// Render the Layers window while `App::layers_dialog` is `Some`.
pub fn draw_layers_dialog(ctx: &egui::Context, app: &mut App) {
    let Some(mut draft) = app.layers_dialog.clone() else {
        return;
    };
    let current = app.document.current_layer();
    let selected_count = app.document.selection.len();
    let rows: Vec<_> = app
        .document
        .layers()
        .iter()
        .map(|l| {
            let count = app.document.layer_entity_count(l.id);
            (l.clone(), count)
        })
        .collect();
    let mut click = None;
    let mut open = true;

    egui::Window::new("Layers")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(LIST_MAX_HEIGHT)
                .show(ui, |ui| {
                    egui::Grid::new("layers_list").striped(true).show(ui, |ui| {
                        for (layer, count) in &rows {
                            swatch(ui, layer.color);
                            let chosen = layer.id == draft.selected;
                            if ui.selectable_label(chosen, &layer.name).clicked() {
                                click = Some(Click::Select(layer.id));
                            }
                            ui.label(if layer.id == current { "current" } else { "" });
                            ui.label(if layer.output { "Output" } else { "No output" });
                            ui.label(format!("Entities: {count}"));
                            ui.end_row();
                        }
                    });
                });
            ui.separator();
            egui::Grid::new("layers_fields").show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut draft.name);
                ui.end_row();
                ui.label("Color");
                ui.color_edit_button_srgb(&mut draft.color);
                ui.end_row();
                ui.label("Output");
                ui.checkbox(&mut draft.output, "Export this layer");
                ui.end_row();
            });
            if !draft.message.is_empty() {
                let colour = ui.visuals().warn_fg_color;
                ui.colored_label(colour, &draft.message);
            }
            ui.horizontal(|ui| {
                let mut button = |label: &str, action: Click| {
                    if ui.button(label).clicked() {
                        click = Some(action);
                    }
                };
                button("Apply", Click::Apply);
                button("New", Click::New);
                button("Delete", Click::Delete);
                button("Set Current", Click::SetCurrent);
            });
            ui.horizontal(|ui| {
                let label = format!("Move Selection Here ({selected_count})");
                if ui.button(label).clicked() {
                    click = Some(Click::MoveSelection);
                }
                if ui.button("Close").clicked() {
                    click = Some(Click::Close);
                }
            });
        });

    app.layers_dialog = Some(draft);
    if !open {
        click = Some(Click::Close);
    }
    match click {
        Some(Click::Select(id)) => app.layers_select(id),
        Some(Click::New) => app.layers_add(),
        Some(Click::Apply) => app.layers_apply(),
        Some(Click::Delete) => app.layers_delete(),
        Some(Click::SetCurrent) => app.layers_make_current(),
        Some(Click::MoveSelection) => app.layers_move_selection(),
        Some(Click::Close) => app.close_layers_dialog(),
        None => {}
    }
}

/// A filled square in the layer's color.
fn swatch(ui: &mut egui::Ui, [r, g, b]: [u8; 3]) {
    let size = egui::vec2(SWATCH_SIZE, SWATCH_SIZE);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(r, g, b));
}
