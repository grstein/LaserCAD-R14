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
use crate::io::svg::matrix::parse_transform;
use crate::io::svg::path_data::parse_path_data;
use crate::io::svg::viewport::{Ctx, nested};

/// The report label of a `<path>` with no `d` (LCV-171 AC 6).
const UNSUPPORTED_PATH: &str = "path (unsupported data)";

/// The report label of a `<path>` whose `d` has a syntax error (LCV-172 AC 8).
const PATH_DATA_ERROR: &str = "path (data error)";

/// The report label of an unparseable `transform` (LCV-173 AC 8).
const INVALID_TRANSFORM: &str = "transform (invalid)";

/// The report label of a `transform` that collapses the plane (LCV-173).
const SINGULAR_TRANSFORM: &str = "transform (singular)";

/// The report label of a circle under a non-similarity map (LCV-173 AC 7).
const NON_UNIFORM_CIRCLE: &str = "circle (non-uniform transform)";

/// The report label of a nested `<svg>`, imported without clipping
/// (LCV-173 AC 9).
const UNCLIPPED_SVG: &str = "svg (not clipped)";

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
    /// layer group; `None` = outside any, which means the first layer), with
    /// `ctx` the context of `node`'s children.
    pub(super) fn collect(
        &mut self,
        node: roxmltree::Node<'_, '_>,
        layer: Option<LayerId>,
        ctx: &Ctx,
    ) -> Result<(), SvgImportError> {
        let bed_h = self.bed_h;
        for child in node.children().filter(|n| n.is_element()) {
            let name = child.tag_name().name();
            let kind = classify(child);
            let mut inner_ctx = None;
            if matches!(kind, Kind::Import | Kind::Descend) {
                self.report.note_properties(child);
                inner_ctx = self.local(child, ctx);
            }
            if matches!(kind, Kind::Descend) && name == "svg" {
                inner_ctx = inner_ctx.map(|c| nested(child, &c));
                self.report.note(UNCLIPPED_SVG);
            }
            let entity = match kind {
                Kind::Import => match (inner_ctx, name) {
                    (None, _) => None,
                    (Some(c), "line") => Some(parse_line(child, &c, bed_h)?),
                    (Some(c), "circle") => {
                        let circle = parse_circle(child, &c, bed_h)?;
                        if circle.is_none() {
                            self.report.note(NON_UNIFORM_CIRCLE);
                        }
                        circle
                    }
                    (Some(c), _) => {
                        self.path(child, layer, &c);
                        None
                    }
                },
                Kind::Descend => {
                    let inner = self.layers.enter(child)?.or(layer);
                    if let Some(c) = inner_ctx {
                        self.collect(child, inner, &c)?;
                    }
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

    /// The context of `node`'s content: its `transform` composed inside
    /// `ctx` (LCV-173 AC 5). An unparseable `transform` counts as absent and
    /// is reported (AC 8); a singular result renders nothing, so it is
    /// reported and `None`.
    pub(super) fn local(&mut self, node: roxmltree::Node<'_, '_>, ctx: &Ctx) -> Option<Ctx> {
        let Some(raw) = node.attribute("transform") else {
            return Some(*ctx);
        };
        let Some(m) = parse_transform(raw) else {
            self.report.note(INVALID_TRANSFORM);
            return Some(*ctx);
        };
        let ctm = m.then(ctx.ctm);
        if ctm.is_singular() {
            self.report.note(SINGULAR_TRANSFORM);
            return None;
        }
        Some(Ctx { ctm, ..*ctx })
    }

    /// Import a `<path>`: every entity its `d` draws, every report label
    /// (LCV-172 AC 2, AC 7, AC 8).
    fn path(&mut self, node: roxmltree::Node<'_, '_>, layer: Option<LayerId>, ctx: &Ctx) {
        let Some(d) = node.attribute("d") else {
            self.report.note(UNSUPPORTED_PATH);
            return;
        };
        let data = parse_path_data(d);
        let (entities, labels) = path_entities(&data, ctx, self.bed_h);
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
