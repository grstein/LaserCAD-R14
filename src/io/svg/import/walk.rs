//! The depth-first walk behind [`super::import_svg`]: which elements become
//! entities, which are descended into, which are reported (LCV-171), and the
//! layer each entity lands on.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::path::path_entities;
use super::report::Report;
use super::{SVG_NS, SvgImportError, parse_circle, parse_line};
use crate::document::LayerId;
use crate::document::entity::Entity;
use crate::io::svg::layers::{LayerReader, STRAY_LAYER};
use crate::io::svg::path_data::parse_path_data;

/// The report label of a `<path>` with no `d` (LCV-171 AC 6).
const UNSUPPORTED_PATH: &str = "path (unsupported data)";

/// The report label of a `<path>` whose `d` has a syntax error (LCV-172 AC 8).
const PATH_DATA_ERROR: &str = "path (data error)";

/// What the walk does with one element (the table in [`super`]'s docs).
enum Kind {
    /// `line`, `circle`, `path`: turned into an entity.
    Import,
    /// `svg`, `g`, `a`: its children are walked (AC 2).
    Descend,
    /// Never rendered: nothing inside is imported (AC 3).
    NeverRendered,
    /// `title`, `desc`, `metadata`, or outside the SVG namespace (AC 4).
    Silent,
    /// Any other SVG element: skipped with its subtree (AC 5).
    Other,
}

/// Classify `node` by namespace and local name.
fn classify(node: roxmltree::Node<'_, '_>) -> Kind {
    if node.tag_name().namespace() != Some(SVG_NS) {
        return Kind::Silent;
    }
    match node.tag_name().name() {
        "line" | "circle" | "path" => Kind::Import,
        "svg" | "g" | "a" => Kind::Descend,
        "defs" | "symbol" | "clipPath" | "mask" | "marker" | "pattern" | "linearGradient"
        | "radialGradient" | "filter" => Kind::NeverRendered,
        "title" | "desc" | "metadata" => Kind::Silent,
        _ => Kind::Other,
    }
}

/// Traversal state: geometry and membership so far, layers so far.
pub(super) struct Walk {
    pub(super) entities: Vec<Entity>,
    pub(super) entity_layers: Vec<LayerId>,
    pub(super) layers: LayerReader,
    pub(super) report: Report,
    bed_h: f64,
}

impl Walk {
    /// An empty walk that un-mirrors Y around `bed_h`.
    pub(super) fn new(bed_h: f64) -> Self {
        Self {
            entities: Vec::new(),
            entity_layers: Vec::new(),
            layers: LayerReader::default(),
            report: Report::default(),
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
            let name = child.tag_name().name();
            let kind = classify(child);
            if matches!(kind, Kind::Import | Kind::Descend) {
                self.report.note_properties(child);
            }
            let entity = match kind {
                Kind::Import => match name {
                    "line" => Some(parse_line(child, bed_h)?),
                    "circle" => Some(parse_circle(child, bed_h)?),
                    _ => {
                        self.path(child, layer);
                        None
                    }
                },
                Kind::Descend => {
                    let inner = self.layers.enter(child)?.or(layer);
                    self.collect(child, inner)?;
                    None
                }
                Kind::NeverRendered => {
                    if child.children().any(|n| n.is_element()) {
                        self.report.note(name);
                    }
                    None
                }
                Kind::Silent => None,
                Kind::Other => {
                    self.report.note(name);
                    None
                }
            };
            if let Some(entity) = entity {
                self.push(entity, layer);
            }
        }
        Ok(())
    }

    /// Import a `<path>`: every entity its `d` draws, every report label
    /// (LCV-172 AC 2, AC 7, AC 8).
    fn path(&mut self, node: roxmltree::Node<'_, '_>, layer: Option<LayerId>) {
        let Some(d) = node.attribute("d") else {
            self.report.note(UNSUPPORTED_PATH);
            return;
        };
        let data = parse_path_data(d);
        let (entities, labels) = path_entities(&data, self.bed_h);
        for entity in entities {
            self.push(entity, layer);
        }
        for label in labels {
            self.report.note(label);
        }
        if data.error {
            self.report.note(PATH_DATA_ERROR);
        }
    }

    fn push(&mut self, entity: Entity, layer: Option<LayerId>) {
        self.entities.push(entity);
        self.entity_layers.push(layer.unwrap_or(STRAY_LAYER));
    }
}
