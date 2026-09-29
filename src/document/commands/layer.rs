//! Layer commands (LCV-156, ADR 0012 §3): [`AddLayer`], [`EditLayer`],
//! [`DeleteLayer`], [`SetCurrentLayer`], [`SetEntityLayers`].
//!
//! Each is one undo step (AC 8). Validation (AC 6/7) is the caller's job via
//! `Document::check_*` before commit; the commands `debug_assert` it and
//! never refuse.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::Command;
use crate::document::{Document, Layer, LayerId};

/// Append a new layer at the end of the layer list.
#[derive(Debug)]
pub struct AddLayer {
    name: String,
    color: [u8; 3],
    output: bool,
    /// Allocated on the first `do_` and kept, so redo re-creates the same id.
    id: Option<LayerId>,
}

impl AddLayer {
    /// A layer named `name` in `color`, Output as given.
    pub fn new(name: impl Into<String>, color: [u8; 3], output: bool) -> Self {
        let name = name.into();
        Self {
            name,
            color,
            output,
            id: None,
        }
    }

    /// The id the layer got, once `do_` has run.
    pub fn id(&self) -> Option<LayerId> {
        self.id
    }
}

impl Command for AddLayer {
    fn do_(&mut self, doc: &mut Document) {
        debug_assert!(doc.check_new_layer(&self.name, self.color).is_ok());
        let id = *self.id.get_or_insert(doc.next_layer_id());
        let layer = Layer {
            id,
            name: self.name.clone(),
            color: self.color,
            output: self.output,
        };
        doc.insert_layer(doc.layers().len(), layer);
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(id) = self.id {
            doc.remove_layer(id);
        }
    }

    fn label(&self) -> &str {
        "Add Layer"
    }
}

/// Replace a layer's name, color and Output flag (matched by `id`).
#[derive(Debug)]
pub struct EditLayer {
    new: Layer,
    old: Option<Layer>,
}

impl EditLayer {
    /// Install `layer` over the existing layer with the same id.
    pub fn new(layer: Layer) -> Self {
        Self {
            new: layer,
            old: None,
        }
    }
}

impl Command for EditLayer {
    fn do_(&mut self, doc: &mut Document) {
        debug_assert!(
            doc.check_edit_layer(self.new.id, &self.new.name, self.new.color)
                .is_ok()
        );
        self.old = doc.replace_layer(self.new.clone());
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(old) = self.old.take() {
            doc.replace_layer(old);
        }
    }

    fn label(&self) -> &str {
        "Edit Layer"
    }
}

/// Remove an empty layer that is not the last one. A deleted current layer
/// hands "current" to the first remaining layer.
#[derive(Debug)]
pub struct DeleteLayer {
    id: LayerId,
    /// `(position, layer, current layer before do_)`.
    captured: Option<(usize, Layer, LayerId)>,
}

impl DeleteLayer {
    /// Delete the layer `id`.
    pub fn new(id: LayerId) -> Self {
        Self { id, captured: None }
    }
}

impl Command for DeleteLayer {
    fn do_(&mut self, doc: &mut Document) {
        debug_assert!(doc.check_delete_layer(self.id).is_ok());
        let prior_current = doc.current_layer();
        let Some((pos, layer)) = doc.remove_layer(self.id) else {
            return;
        };
        if prior_current == self.id {
            let first = doc.layers()[0].id; // Invariant: another layer remains.
            doc.set_current_layer(first);
        }
        self.captured = Some((pos, layer, prior_current));
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some((pos, layer, prior_current)) = self.captured.take() {
            doc.insert_layer(pos, layer);
            doc.set_current_layer(prior_current);
        }
    }

    fn label(&self) -> &str {
        "Delete Layer"
    }
}

/// Make a layer the one new entities land on.
#[derive(Debug)]
pub struct SetCurrentLayer {
    id: LayerId,
    previous: Option<LayerId>,
}

impl SetCurrentLayer {
    /// Make `id` current.
    pub fn new(id: LayerId) -> Self {
        Self { id, previous: None }
    }
}

impl Command for SetCurrentLayer {
    fn do_(&mut self, doc: &mut Document) {
        self.previous = Some(doc.set_current_layer(self.id));
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(previous) = self.previous.take() {
            doc.set_current_layer(previous);
        }
    }

    fn label(&self) -> &str {
        "Set Current Layer"
    }
}

/// Move entities onto one layer.
#[derive(Debug)]
pub struct SetEntityLayers {
    indices: Vec<usize>,
    layer: LayerId,
    /// `(index, previous layer)` in `indices` order.
    previous: Vec<(usize, LayerId)>,
}

impl SetEntityLayers {
    /// Move the entities at `indices` onto `layer`.
    pub fn new(indices: Vec<usize>, layer: LayerId) -> Self {
        Self {
            indices,
            layer,
            previous: Vec::new(),
        }
    }
}

impl Command for SetEntityLayers {
    fn do_(&mut self, doc: &mut Document) {
        debug_assert!(
            doc.layer(self.layer).is_some(),
            "SetEntityLayers: unknown layer"
        );
        self.previous = self
            .indices
            .iter()
            .map(|&i| (i, doc.set_entity_layer(i, self.layer)))
            .collect();
    }

    fn undo(&mut self, doc: &mut Document) {
        for (i, previous) in self.previous.drain(..).rev() {
            doc.set_entity_layer(i, previous);
        }
    }

    fn label(&self) -> &str {
        "Move to Layer"
    }
}
