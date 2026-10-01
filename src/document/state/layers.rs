//! Layer-editing half of [`Document`] (LCV-156, ADR 0012): the checks the
//! layer commands run and the `pub(crate)` writes they make. Split from
//! `state.rs` for the ADR 0004 cap (LCV-188).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::document::layer::check_fields;
use crate::document::{Document, Layer, LayerError, LayerId};

impl Document {
    /// Refuse a new layer named `name` with `color` that would break the
    /// layer invariants (AC 6).
    pub fn check_new_layer(&self, name: &str, color: [u8; 3]) -> Result<(), LayerError> {
        check_fields(&self.layers, None, name, color)
    }

    /// Refuse renaming/recoloring layer `id` to `name`/`color` (AC 6).
    pub fn check_edit_layer(
        &self,
        id: LayerId,
        name: &str,
        color: [u8; 3],
    ) -> Result<(), LayerError> {
        if self.layer(id).is_none() {
            return Err(LayerError::UnknownLayer(format!("#{}", id.0)));
        }
        check_fields(&self.layers, Some(id), name, color)
    }

    /// Refuse deleting layer `id` while it has entities or is the last (AC 7).
    pub fn check_delete_layer(&self, id: LayerId) -> Result<(), LayerError> {
        let layer = self
            .layer(id)
            .ok_or(LayerError::UnknownLayer(format!("#{}", id.0)))?;
        if self.layer_entity_count(id) > 0 {
            return Err(LayerError::NotEmpty(layer.name.clone()));
        }
        if self.layers.len() == 1 {
            return Err(LayerError::LastLayer);
        }
        Ok(())
    }

    /// Insert `layer` at display position `pos` (layer commands only).
    pub(crate) fn insert_layer(&mut self, pos: usize, layer: Layer) {
        self.layers.insert(pos.min(self.layers.len()), layer);
    }

    /// Remove layer `id`, returning it and its display position.
    pub(crate) fn remove_layer(&mut self, id: LayerId) -> Option<(usize, Layer)> {
        let pos = self.layers.iter().position(|l| l.id == id)?;
        Some((pos, self.layers.remove(pos)))
    }

    /// Replace the layer with `layer.id`, returning the old value.
    pub(crate) fn replace_layer(&mut self, layer: Layer) -> Option<Layer> {
        let slot = self.layers.iter_mut().find(|l| l.id == layer.id)?;
        Some(std::mem::replace(slot, layer))
    }

    /// Make `id` current, returning the previous current layer.
    pub(crate) fn set_current_layer(&mut self, id: LayerId) -> LayerId {
        debug_assert!(self.layer(id).is_some(), "set_current_layer: unknown layer");
        std::mem::replace(&mut self.current_layer, id)
    }

    /// Move entity `index` onto `layer`, returning its previous layer.
    pub(crate) fn set_entity_layer(&mut self, index: usize, layer: LayerId) -> LayerId {
        std::mem::replace(&mut self.entity_layers[index], layer)
    }
}
