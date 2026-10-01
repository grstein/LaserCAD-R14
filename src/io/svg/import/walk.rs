//! The depth-first walk behind [`super::import_svg`]: which elements become
//! entities, which are descended into, which are reported (LCV-171), and the
//! layer each entity lands on.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::path::path_entities;
use super::report::{Report, style_decls};
use super::shapes::import_shape;
use super::style::Style;
use super::{SVG_NS, SvgImportError};
use crate::document::LayerId;
use crate::document::entity::Entity;
use crate::io::svg::css::Sheet;
use crate::io::svg::layers::{LayerReader, Slot};
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

/// The report label of a nested `<svg>`, imported without clipping
/// (LCV-173 AC 9).
const UNCLIPPED_SVG: &str = "svg (not clipped)";

/// What the walk does with one element (the table in [`super`]'s docs).
enum Kind {
    /// `line`, `circle`, `ellipse`, `rect`, `path`: turned into entities.
    Import,
    /// `svg`, `g`, `a`: its children are walked (AC 2).
    Descend,
    /// Never rendered: nothing inside is imported (AC 3).
    NeverRendered,
    /// `title`, `desc`, `metadata`, `style` (read by the cascade), or
    /// outside the SVG namespace (AC 4).
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
        "line" | "circle" | "ellipse" | "rect" | "path" => Kind::Import,
        "svg" | "g" | "a" => Kind::Descend,
        "defs" | "symbol" | "clipPath" | "mask" | "marker" | "pattern" | "linearGradient"
        | "radialGradient" | "filter" => Kind::NeverRendered,
        "title" | "desc" | "metadata" | "style" => Kind::Silent,
        _ => Kind::Other,
    }
}

/// Whether `node` is an SVG `<style>`: a never-rendered element holding
/// only those is not reported (LCV-175).
fn is_style(node: roxmltree::Node<'_, '_>) -> bool {
    node.tag_name().name() == "style" && node.tag_name().namespace() == Some(SVG_NS)
}

/// Traversal state: geometry and membership so far, layers so far.
pub(super) struct Walk {
    pub(super) entities: Vec<Entity>,
    /// Where each entity belongs, resolved by [`LayerReader::finish`].
    pub(super) slots: Vec<Slot>,
    pub(super) layers: LayerReader,
    pub(super) report: Report,
    /// The document's `<style>` rules (LCV-175).
    pub(super) sheet: Sheet,
    bed_h: f64,
}

impl Walk {
    /// An empty walk that un-mirrors Y around `bed_h`.
    pub(super) fn new(bed_h: f64) -> Self {
        Self {
            entities: Vec::new(),
            slots: Vec::new(),
            layers: LayerReader::default(),
            report: Report::default(),
            sheet: Sheet::default(),
            bed_h,
        }
    }

    /// Append `node`'s recognised geometry on `layer` (the innermost enclosing
    /// layer group; `None` = outside any, which means the first layer), with
    /// `ctx` the context and `style` the computed style of `node`.
    pub(super) fn collect(
        &mut self,
        node: roxmltree::Node<'_, '_>,
        layer: Option<LayerId>,
        ctx: &Ctx,
        style: &Style,
    ) -> Result<(), SvgImportError> {
        let bed_h = self.bed_h;
        for child in node.children().filter(|n| n.is_element()) {
            let name = child.tag_name().name();
            let kind = classify(child);
            let mut inner_ctx = None;
            let mut inner_style = *style;
            if matches!(kind, Kind::Import | Kind::Descend) {
                inner_style = style.child(child, &self.sheet, &mut self.report);
                if let Some(label) = inner_style.hidden(matches!(kind, Kind::Import)) {
                    self.layers.enter(child)?; // A hidden layer group still declares its layer.
                    self.report.note(label);
                    continue;
                }
                self.report.note_properties(child);
                inner_ctx = self.local(child, ctx);
            }
            if let (Kind::Descend, "svg", Some(c)) = (&kind, name, inner_ctx) {
                inner_ctx = nested(child, &c);
                let label = match inner_ctx {
                    Some(_) => UNCLIPPED_SVG,
                    None => SINGULAR_TRANSFORM,
                };
                self.report.note(label);
            }
            let slot = inner_style.slot(layer);
            let entity = match kind {
                Kind::Import => match (inner_ctx, name) {
                    (None, _) => None,
                    (Some(c), "path") => {
                        self.path(child, slot, &c);
                        None
                    }
                    (Some(c), _) => {
                        let shape = import_shape(name, child, &c, bed_h);
                        self.push(shape.entities, &shape.notes, slot);
                        None
                    }
                },
                Kind::Descend => {
                    let inner = self.layers.enter(child)?.or(layer);
                    if let Some(c) = inner_ctx {
                        self.collect(child, inner, &c, &inner_style)?;
                    }
                    None
                }
                Kind::NeverRendered => {
                    if child.children().any(|n| n.is_element() && !is_style(n)) {
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
                self.slots.push(slot);
                self.entities.push(entity);
            }
        }
        Ok(())
    }

    /// The context of `node`'s content: its `transform` composed inside
    /// `ctx` (LCV-173 AC 5). The last `transform` style declaration (ASCII
    /// case-insensitive) wins over the attribute, and CSS `none` is no
    /// transform. An unparseable `transform` counts as absent and is
    /// reported (AC 8); a singular result renders nothing, so it is reported
    /// and `None`.
    pub(super) fn local(&mut self, node: roxmltree::Node<'_, '_>, ctx: &Ctx) -> Option<Ctx> {
        let styled = style_decls(node)
            .rev()
            .find(|(prop, _)| prop.eq_ignore_ascii_case("transform"))
            .map(|(_, value)| value);
        let raw = match styled {
            Some(css) if css.eq_ignore_ascii_case("none") => return Some(*ctx),
            Some(css) => css,
            None => match node.attribute("transform") {
                Some(attr) => attr,
                None => return Some(*ctx),
            },
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
    fn path(&mut self, node: roxmltree::Node<'_, '_>, slot: Slot, ctx: &Ctx) {
        let Some(d) = node.attribute("d") else {
            self.report.note(UNSUPPORTED_PATH);
            return;
        };
        let data = parse_path_data(d);
        let (entities, labels) = path_entities(&data, ctx, self.bed_h);
        self.push(entities, &labels, slot);
        if data.error {
            self.report.note(PATH_DATA_ERROR);
        }
    }

    /// Append `entities` on `slot` and note each of `labels`.
    fn push(&mut self, entities: Vec<Entity>, labels: &[&str], slot: Slot) {
        self.slots.extend(entities.iter().map(|_| slot));
        self.entities.extend(entities);
        for label in labels {
            self.report.note(label);
        }
    }
}
