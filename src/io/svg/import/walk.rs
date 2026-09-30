//! The depth-first walk behind [`super::import_svg`]: which elements become
//! entities, which are descended into, and the layer each entity lands on.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{SvgImportError, parse_circle, parse_line, parse_path};
use crate::document::LayerId;
use crate::document::entity::Entity;
use crate::io::svg::layers::{LayerReader, STRAY_LAYER};

/// Traversal state: geometry and membership so far, layers so far.
pub(super) struct Walk {
    pub(super) entities: Vec<Entity>,
    pub(super) entity_layers: Vec<LayerId>,
    pub(super) layers: LayerReader,
    bed_h: f64,
}

impl Walk {
    /// An empty walk that un-mirrors Y around `bed_h`.
    pub(super) fn new(bed_h: f64) -> Self {
        Self {
            entities: Vec::new(),
            entity_layers: Vec::new(),
            layers: LayerReader::default(),
            bed_h,
        }
    }

    /// Append `node`'s recognised geometry on `layer` (the innermost enclosing
    /// layer group; `None` = outside any, which means the first layer).
    pub(super) fn collect(
        &mut self,
        node: roxmltree::Node<'_, '_>,
        layer: Option<LayerId>,
    ) -> Result<(), SvgImportError> {
        let bed_h = self.bed_h;
        for child in node.children().filter(|n| n.is_element()) {
            let entity = match child.tag_name().name() {
                "line" => Some(parse_line(child, bed_h)?),
                "circle" => Some(parse_circle(child, bed_h)?),
                "path" => parse_path(child, bed_h)?,
                _ => {
                    let inner = self.layers.enter(child)?.or(layer);
                    self.collect(child, inner)?;
                    None
                }
            };
            if let Some(entity) = entity {
                self.entities.push(entity);
                self.entity_layers.push(layer.unwrap_or(STRAY_LAYER));
            }
        }
        Ok(())
    }
}
