//! SVG `<text>` import as glyph outlines (LCV-179, ADR 0017): the text and
//! its `tspan`/`a` descendants are flattened into characters, each with its
//! face, size and layer slot, laid out by horizontal advances (no shaping,
//! ligatures or kerning) and drawn as the lines and Béziers of their
//! glyphs, through the element's transform.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::SVG_NS;
use super::path::path_entities;
use super::style::{Style, declared};
use super::walk::Walk;
use crate::document::LayerId;
use crate::geometry::Vec2;
use crate::io::svg::css::Sheet;
use crate::io::svg::layers::Slot;
use crate::io::svg::length::{font_size, parse_length, to_user};
use crate::io::svg::path_data::{PathData, Segment};
use crate::io::svg::viewport::Ctx;
use crate::text::{FaceId, Glyph, Seg, glyph};

type Node<'a, 'input> = roxmltree::Node<'a, 'input>;

/// The report label of a `<text>` skipped because no font is installed.
const NO_FONT: &str = "text (no font)";

/// The initial `font-size` in user units (CSS `medium`).
const DEFAULT_SIZE: f64 = 16.0;

/// One addressable character.
#[derive(Debug, Clone, Copy)]
struct Char {
    c: char,
    face: FaceId,
    /// `font-size` in user units.
    size: f64,
    slot: Slot,
    /// Visible and painted: its outline is imported.
    drawn: bool,
}

/// A laid-out element's characters `first..end` and its `x`, `y`, `dx`,
/// `dy` lists in user units.
type Span = (usize, usize, [Vec<f64>; 4]);

/// The characters of one `<text>` and its elements' spans, in document
/// order (an element before its descendants).
#[derive(Debug, Default)]
struct Flat {
    chars: Vec<Char>,
    spans: Vec<Span>,
}

impl<'a, 'input> Walk<'a, 'input> {
    /// Import the `<text>` `node` with context `ctx` (its transform
    /// composed) and computed style `style`, on `layer`.
    pub(super) fn text(
        &mut self,
        node: Node<'a, 'input>,
        layer: Option<LayerId>,
        ctx: &Ctx,
        style: &Style,
    ) {
        if self.fonts.face("sans-serif", 400, false).is_none() {
            self.report.note(NO_FONT);
            return;
        }
        let mut flat = Flat::default();
        self.flatten(node, layer, ctx, style, &mut flat);
        let Some(glyphs) = self.glyphs(&flat.chars) else {
            self.report.note(NO_FONT);
            return;
        };
        let origins = layout(&glyphs, &positions(flat.chars.len(), &flat.spans));
        for ((ch, (g, s)), o) in flat.chars.iter().zip(&glyphs).zip(origins) {
            if !ch.drawn {
                continue;
            }
            let at = |p: Vec2| Vec2::new(o.x + p.x * s, o.y - p.y * s);
            let segments = g.contours.iter().flatten().map(|seg| match *seg {
                Seg::Line(a, b) => Segment::Line(at(a), at(b)),
                Seg::Quad(p) => Segment::Quad(p.map(at)),
                Seg::Cubic(p) => Segment::Cubic(p.map(at)),
            });
            let data = PathData {
                segments: segments.collect(),
                error: false,
            };
            let (entities, _) = path_entities(&data, ctx, self.bed_h);
            self.push(entities, &[], ch.slot);
        }
    }

    /// Append the characters of `node` (a `<text>` or a laid-out child)
    /// styled `style` to `flat`, descending into `tspan` and `a`.
    fn flatten(
        &mut self,
        node: Node<'a, 'input>,
        layer: Option<LayerId>,
        ctx: &Ctx,
        style: &Style,
        flat: &mut Flat,
    ) {
        let families = inherited(node, &self.sheet, "font-family");
        let weight = weight(inherited(node, &self.sheet, "font-weight").as_deref());
        let italic = inherited(node, &self.sheet, "font-style")
            .is_some_and(|v| v.starts_with("italic") || v.starts_with("oblique"));
        let families = families.as_deref().unwrap_or("sans-serif");
        let Some((face, _)) = self.fonts.face(families, weight, italic) else {
            return;
        };
        let template = Char {
            c: ' ',
            face,
            size: size(node, &self.sheet),
            slot: style.slot(layer),
            drawn: !style.invisible,
        };
        let span = flat.spans.len();
        let lists = ["x", "y", "dx", "dy"].map(|name| list(node, name, ctx));
        flat.spans.push((flat.chars.len(), 0, lists));
        for child in node.children() {
            if let Some(text) = child.is_text().then(|| child.text()).flatten() {
                for c in text.chars() {
                    flat.chars.push(Char { c, ..template });
                }
                continue;
            }
            if !child.is_element() || child.tag_name().namespace() != Some(SVG_NS) {
                continue;
            }
            if let "tspan" | "a" = child.tag_name().name() {
                let inner = style.child(child, &self.sheet, &mut self.report);
                if !inner.display_none {
                    self.report.note_properties(child);
                    self.flatten(child, layer, ctx, &inner, flat);
                }
            }
        }
        flat.spans[span].1 = flat.chars.len();
    }

    /// Each character's glyph and font-unit scale (`size / units_per_em`),
    /// reading each face once; `None` when a face cannot be read.
    fn glyphs(&self, chars: &[Char]) -> Option<Vec<(Glyph, f64)>> {
        let mut out = vec![None; chars.len()];
        let mut faces: Vec<FaceId> = chars.iter().map(|ch| ch.face).collect();
        faces.sort();
        faces.dedup();
        for face in faces {
            self.fonts.with_face(face, |f| {
                let upem = f64::from(f.units_per_em());
                for (i, ch) in chars.iter().enumerate().filter(|(_, ch)| ch.face == face) {
                    out[i] = Some((glyph(f, ch.c), ch.size / upem));
                }
            })?;
        }
        out.into_iter().collect()
    }
}

/// Per character of `n`, its `x`, `y`, `dx`, `dy`: the i-th value of an
/// element's list goes to its i-th character, an inner element's list
/// overriding its ancestors' (AC 6).
fn positions(n: usize, spans: &[Span]) -> Vec<[Option<f64>; 4]> {
    let mut pos = vec![[None; 4]; n];
    for (first, end, lists) in spans {
        let chars = pos.get_mut(*first..(*end).min(n)).unwrap_or_default();
        for (k, values) in lists.iter().enumerate() {
            for (p, v) in chars.iter_mut().zip(values) {
                p[k] = Some(*v);
            }
        }
    }
    pos
}

/// Each character's glyph origin in user units: the pen starts at (0, 0),
/// moves to a given `x`/`y`, then by `dx`/`dy`, and advances by the glyph's
/// scaled advance after each character (AC 1, AC 4, AC 6).
fn layout(glyphs: &[(Glyph, f64)], pos: &[[Option<f64>; 4]]) -> Vec<Vec2> {
    let mut pen = Vec2::new(0.0, 0.0);
    let mut origins = Vec::with_capacity(glyphs.len());
    for ((g, s), [x, y, dx, dy]) in glyphs.iter().zip(pos) {
        pen.x = x.unwrap_or(pen.x) + dx.unwrap_or(0.0);
        pen.y = y.unwrap_or(pen.y) + dy.unwrap_or(0.0);
        origins.push(pen);
        pen.x += g.advance * s;
    }
    origins
}

/// The nearest declared value of the inherited property `prop` at `node`
/// or above (`inherit`/`unset` defer to the parent).
fn inherited(node: Node<'_, '_>, sheet: &Sheet, prop: &str) -> Option<String> {
    node.ancestors()
        .filter(|n| n.is_element())
        .filter_map(|n| declared(n, sheet, prop))
        .find(|v| !v.eq_ignore_ascii_case("inherit") && !v.eq_ignore_ascii_case("unset"))
        .map(|v| v.to_ascii_lowercase())
}

/// `font-weight` as a number: 1–1000 as written, `bold`/`bolder` 700,
/// anything else 400.
fn weight(value: Option<&str>) -> u16 {
    match value {
        Some("bold" | "bolder") => 700,
        Some(v) => v
            .parse::<f64>()
            .map_or(400, |w| w.clamp(1.0, 1000.0) as u16),
        None => 400,
    }
}

/// The computed `font-size` of `node` in user units, resolving each
/// ancestor's declaration against its parent's size from [`DEFAULT_SIZE`].
fn size(node: Node<'_, '_>, sheet: &Sheet) -> f64 {
    let chain: Vec<_> = node.ancestors().filter(|n| n.is_element()).collect();
    chain.iter().rev().fold(DEFAULT_SIZE, |parent, n| {
        declared(*n, sheet, "font-size")
            .and_then(|v| font_size(&v, parent))
            .unwrap_or(parent)
    })
}

/// The `x`, `y`, `dx` or `dy` list of `node` in user units (`%` of the
/// viewport width for `x`/`dx`, height for `y`/`dy`), up to the first
/// invalid value.
fn list(node: Node<'_, '_>, name: &str, ctx: &Ctx) -> Vec<f64> {
    let reference = ctx.viewport[usize::from(name.ends_with('y'))];
    node.attribute(name)
        .unwrap_or("")
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|t| !t.is_empty())
        .map_while(|t| parse_length(t).map(|len| to_user(len, reference)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::entity::Entity;
    use crate::io::svg::{ImportedSvg, import_svg_with};
    use crate::text::FontBook;
    use crate::text::fonts::tests::book;

    const UPEM: f64 = 2048.0;

    /// The import of a 100 mm page (1 user unit = 1 mm) holding `inner`,
    /// with `fonts`.
    fn page_with(inner: &str, fonts: &FontBook) -> ImportedSvg {
        let src = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{inner}</svg>"#
        );
        import_svg_with(&src, fonts).unwrap()
    }

    /// [`page_with`] the bundled test font.
    fn page(inner: &str) -> ImportedSvg {
        page_with(inner, &book())
    }

    /// The font-unit bounding box `(min, max)` of `c` in the regular face.
    fn glyph_box(c: char) -> (Vec2, Vec2) {
        let book = book();
        let (id, _) = book.face("LCV Test Sans", 400, false).unwrap();
        let g = book.with_face(id, |f| glyph(f, c)).unwrap();
        let points = g.contours.iter().flatten().flat_map(|s| match *s {
            Seg::Line(a, b) => vec![a, b],
            Seg::Quad(p) => p.to_vec(),
            Seg::Cubic(p) => p.to_vec(),
        });
        bounds(points)
    }

    fn bounds(points: impl Iterator<Item = Vec2>) -> (Vec2, Vec2) {
        let inf = f64::INFINITY;
        points.fold(
            (Vec2::new(inf, inf), Vec2::new(-inf, -inf)),
            |(lo, hi), p| {
                (
                    Vec2::new(lo.x.min(p.x), lo.y.min(p.y)),
                    Vec2::new(hi.x.max(p.x), hi.y.max(p.y)),
                )
            },
        )
    }

    /// The union of the entities' bounding boxes.
    fn entity_box(es: &[Entity]) -> (Vec2, Vec2) {
        bounds(es.iter().flat_map(|e| {
            let (lo, hi) = e.bbox();
            [lo, hi]
        }))
    }

    fn assert_box(got: (Vec2, Vec2), want: (Vec2, Vec2)) {
        let ok = got.0.approx_eq(want.0, 1e-9) && got.1.approx_eq(want.1, 1e-9);
        assert!(ok, "got {got:?}, want {want:?}");
    }

    /// AC 1 — `l` at (10, 20), size 10, under `translate(5 7) scale(2)`:
    /// one closed contour of lines, the glyph box scaled by size/upem with
    /// its origin at (10, 20), through the transform and the Y mirror.
    #[test]
    fn a_glyph_imports_as_closed_contours_at_its_origin() {
        let svg = page(
            r#"<g transform="translate(5 7) scale(2)"><text x="10" y="20" font-size="10">l</text></g>"#,
        );
        let lines: Vec<_> = (svg.entities.iter())
            .map(|e| match e {
                Entity::Line(l) => *l,
                other => panic!("not a line: {other:?}"),
            })
            .collect();
        assert!(!lines.is_empty());
        for (i, l) in lines.iter().enumerate() {
            let next = lines[(i + 1) % lines.len()];
            assert!(l.p2.approx_eq(next.p1, 1e-9), "open at {i}: {lines:?}");
        }
        let (lo, hi) = glyph_box('l');
        let s = 10.0 / UPEM;
        let world = |x: f64, y: f64| Vec2::new(5.0 + 2.0 * x, 100.0 - (7.0 + 2.0 * y));
        let a = world(10.0 + lo.x * s, 20.0 - lo.y * s);
        let b = world(10.0 + hi.x * s, 20.0 - hi.y * s);
        assert_box(entity_box(&svg.entities), bounds([a, b].into_iter()));
        assert!(svg.report.is_empty(), "{:?}", svg.report);
    }

    /// The world box of glyph `c` drawn at SVG point `(x, y)` with scale
    /// `s` on the 100 mm page.
    fn placed_box(c: char, x: f64, y: f64, s: f64) -> (Vec2, Vec2) {
        let (lo, hi) = glyph_box(c);
        let world = |gx: f64, gy: f64| Vec2::new(x + gx * s, 100.0 - (y - gy * s));
        bounds([world(lo.x, lo.y), world(hi.x, hi.y)].into_iter())
    }

    /// AC 4 — the second `l` sits exactly one scaled advance (455) right
    /// of the first.
    #[test]
    fn glyphs_advance_by_their_horizontal_advance() {
        let one = page(r#"<text x="10" y="50" font-size="20.48">l</text>"#).entities;
        let two = page(r#"<text x="10" y="50" font-size="20.48">ll</text>"#).entities;
        assert_eq!(two.len(), 2 * one.len());
        let s = 20.48 / UPEM;
        assert_box(
            entity_box(&two[..one.len()]),
            placed_box('l', 10.0, 50.0, s),
        );
        let second = placed_box('l', 10.0 + 455.0 * s, 50.0, s);
        assert_box(entity_box(&two[one.len()..]), second);
    }

    /// AC 4 — no kerning: `V` after `A` starts at `A`'s full advance
    /// (1366), although the font kerns the pair by −152.
    #[test]
    fn pairs_are_not_kerned() {
        let a = page(r#"<text x="10" y="50" font-size="20.48">A</text>"#).entities;
        let av = page(r#"<text x="10" y="50" font-size="20.48">AV</text>"#).entities;
        let s = 20.48 / UPEM;
        let v = placed_box('V', 10.0 + 1366.0 * s, 50.0, s);
        assert_box(entity_box(&av[a.len()..]), v);
    }
}
