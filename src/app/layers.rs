//! The Layers… dialog's state and decisions (LCV-156 AC 4, 6, 7; ADR 0012 §9).
//!
//! [`LayersDialog`] holds the selected row and its edit buffers; the `App`
//! methods below are what the dialog's buttons call, so a headless test
//! drives them without a pointer. Every change is one [`App::commit`] of a
//! layer command (AC 8); a refused change commits nothing and leaves the
//! operator-facing reason in [`LayersDialog::message`]. The egui half is
//! `src/ui/layers_dialog.rs`.
//!
//! MUST NOT import `egui`, `eframe` or `rfd`.

use super::App;
use crate::document::{
    AddLayer, DeleteLayer, EditLayer, Layer, LayerId, SetCurrentLayer, SetEntityLayers,
};

/// Colors offered to new layers, in order, before any other unused color:
/// the LaserGRBL mark and engrave conventions first.
const NEW_LAYER_COLORS: [[u8; 3]; 6] = [
    [0, 0, 255],
    [0, 170, 0],
    [255, 0, 255],
    [0, 170, 170],
    [255, 128, 0],
    [128, 0, 255],
];

/// The open Layers… dialog: which layer is selected and its edit buffers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayersDialog {
    /// The selected layer; the buffers below were loaded from it.
    pub selected: LayerId,
    /// Name buffer, applied by [`App::layers_apply`].
    pub name: String,
    /// Color buffer, applied by [`App::layers_apply`].
    pub color: [u8; 3],
    /// Output buffer, applied by [`App::layers_apply`].
    pub output: bool,
    /// Why the last change was refused; empty when it was not.
    pub message: String,
}

impl LayersDialog {
    fn of(layer: &Layer) -> Self {
        Self {
            selected: layer.id,
            name: layer.name.clone(),
            color: layer.color,
            output: layer.output,
            message: String::new(),
        }
    }
}

impl App {
    /// Open the dialog on the current layer.
    pub fn open_layers_dialog(&mut self) {
        self.layers_select(self.document.current_layer());
    }

    /// Close the dialog, discarding unapplied edits.
    pub fn close_layers_dialog(&mut self) {
        self.layers_dialog = None;
    }

    /// Select `id`'s row and load its fields into the buffers.
    pub fn layers_select(&mut self, id: LayerId) {
        let layer = self
            .document
            .layer(id)
            .unwrap_or(&self.document.layers()[0]);
        self.layers_dialog = Some(LayersDialog::of(layer));
    }

    /// Add `LayerN` (first free N) in the first unused color, Output on,
    /// and select it.
    pub fn layers_add(&mut self) {
        let doc = &self.document;
        let taken = |name: &str| doc.layer_by_name(name).is_some();
        let name = (1..)
            .map(|n| format!("Layer{n}"))
            .find(|n| !taken(n))
            .unwrap_or_default();
        let used = |c: &[u8; 3]| doc.layers().iter().any(|l| l.color == *c);
        let color = NEW_LAYER_COLORS
            .into_iter()
            .chain((0..0x0100_0000u32).map(|k| {
                let [_, r, g, b] = k.to_be_bytes();
                [r, g, b]
            }))
            .find(|c| !used(c))
            .unwrap_or([0, 0, 0]);
        // `AddLayer::do_` allocates `next_layer_id()`; read it first.
        let id = doc.next_layer_id();
        self.commit(Box::new(AddLayer::new(name, color, true)));
        self.layers_select(id);
    }

    /// Apply the buffers to the selected layer; refused on a duplicate or
    /// unusable name or a duplicate color (AC 6). Unchanged buffers commit
    /// nothing.
    pub fn layers_apply(&mut self) {
        let Some(d) = self.layers_dialog.clone() else {
            return;
        };
        let new = Layer {
            id: d.selected,
            name: d.name.trim().to_owned(),
            color: d.color,
            output: d.output,
        };
        if self.document.layer(d.selected) == Some(&new) {
            return self.layers_refuse("");
        }
        match self.document.check_edit_layer(new.id, &new.name, new.color) {
            Ok(()) => {
                self.commit(Box::new(EditLayer::new(new)));
                self.layers_select(d.selected);
            }
            Err(e) => self.layers_refuse(&e.to_string()),
        }
    }

    /// Delete the selected layer; refused while it has entities or is the
    /// last one (AC 7). The first layer is selected afterwards.
    pub fn layers_delete(&mut self) {
        let Some(id) = self.layers_dialog.as_ref().map(|d| d.selected) else {
            return;
        };
        match self.document.check_delete_layer(id) {
            Ok(()) => {
                self.commit(Box::new(DeleteLayer::new(id)));
                self.layers_select(self.document.layers()[0].id);
            }
            Err(e) => self.layers_refuse(&e.to_string()),
        }
    }

    /// Make the selected layer current (one undo step when it changes).
    pub fn layers_make_current(&mut self) {
        let Some(id) = self.layers_dialog.as_ref().map(|d| d.selected) else {
            return;
        };
        if id != self.document.current_layer() {
            self.commit(Box::new(SetCurrentLayer::new(id)));
        }
        self.layers_refuse("");
    }

    /// Move the selected entities onto the selected layer.
    pub fn layers_move_selection(&mut self) {
        let Some(id) = self.layers_dialog.as_ref().map(|d| d.selected) else {
            return;
        };
        let indices: Vec<usize> = self.document.selection.iter().collect();
        if indices.is_empty() {
            return self.layers_refuse("Select entities first, then move them to a layer.");
        }
        self.commit(Box::new(SetEntityLayers::new(indices, id)));
        self.layers_refuse("");
    }

    /// Set the dialog's message (empty clears it).
    fn layers_refuse(&mut self, message: &str) {
        if let Some(d) = self.layers_dialog.as_mut() {
            d.message = message.to_owned();
        }
    }
}
