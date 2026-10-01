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
use crate::io::svg::length::font_size;
use crate::io::svg::path_data::{PathData, Segment};
use crate::io::svg::viewport::Ctx;
use crate::text::{FaceId, Glyph, Seg, glyph};

use layout::{Span, collapse, layout, list, positions};

mod layout;

use roxmltree::NS_XML_URI;

type Node<'a, 'input> = roxmltree::Node<'a, 'input>;

/// The report label of a `<text>` skipped because no font is installed.
const NO_FONT: &str = "text (no font)";

/// The report label of a `<text>` drawn in the default face because no
/// listed family is installed, once per text (AC 3).
const SUBSTITUTED: &str = "text (font substituted)";

/// The report label of a character without a glyph, per character (AC 8).
const MISSING: &str = "text (missing glyph)";

/// Properties laid out as if absent and reported `text (<name>)` once per
/// `<text>` when not neutral (AC 9).
const IGNORED: [&str; 4] = ["rotate", "inline-size", "letter-spacing", "word-spacing"];

/// The initial `font-size` in user units (CSS `medium`).
const DEFAULT_SIZE: f64 = 16.0;

/// One addressable character.
#[derive(Debug, Clone, Copy)]
struct Char {
    c: char,
    face: FaceId,
    /// `face` stands in for an uninstalled family.
    substituted: bool,
    /// `font-size` in user units.
    size: f64,
    slot: Slot,
    /// `text-anchor` as a fraction of the chunk width: 0, ½ or 1.
    anchor: f64,
    /// Visible and painted (a stroke, or a fill not `none`): its outline
    /// is imported (AC 10).
    drawn: bool,
}

/// The characters of one `<text>` and its elements' spans, in document
/// order (an element before its descendants).
#[derive(Debug, Default)]
struct Flat {
    chars: Vec<Char>,
    spans: Vec<Span>,
    /// The next collapsible whitespace is dropped: the last character
    /// kept is a collapsed space, or none is kept yet.
    collapse: bool,
}

impl Flat {
    /// Append `ch`, a whitespace becoming a space; a run of collapsible
    /// whitespace keeps one space, none at the start (AC 7).
    fn push(&mut self, ch: Char, preserve: bool) {
        if let Some(c) = collapse(ch.c, preserve, &mut self.collapse) {
            self.chars.push(Char { c, ..ch });
        }
    }

    /// Drop a trailing collapsed space (AC 7).
    fn trim_end(&mut self) {
        if self.collapse {
            self.chars.pop();
        }
    }
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
        if let Some(label) = self.skipped(node) {
            self.report.note(label);
            return;
        }
        for name in IGNORED {
            let used = (node.descendants().filter(|n| n.is_element()))
                .any(|n| inherited(n, &self.sheet, name).is_some_and(|v| !neutral(&v)));
            if used {
                self.report.note(&format!("text ({name})"));
            }
        }
        let mut flat = Flat {
            collapse: true,
            ..Flat::default()
        };
        self.flatten(node, layer, ctx, style, &mut flat);
        flat.trim_end();
        let Some(glyphs) = self.glyphs(&flat.chars) else {
            self.report.note(NO_FONT);
            return;
        };
        let advances: Vec<_> = glyphs.iter().map(|(g, s)| g.advance * s).collect();
        let anchors: Vec<_> = flat.chars.iter().map(|ch| ch.anchor).collect();
        let pos = positions(flat.chars.len(), &flat.spans);
        let origins = layout(&advances, &anchors, &pos);
        if flat.chars.iter().any(|ch| ch.substituted) {
            self.report.note(SUBSTITUTED);
        }
        for ((ch, (g, s)), o) in flat.chars.iter().zip(&glyphs).zip(origins) {
            if !ch.drawn {
                continue;
            }
            if g.missing {
                self.report.note(MISSING);
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
        let Some((face, substituted)) = self.fonts.face(families, weight, italic) else {
            return;
        };
        let template = Char {
            c: ' ',
            face,
            substituted,
            size: size(node, &self.sheet),
            slot: style.slot(layer),
            anchor: match inherited(node, &self.sheet, "text-anchor").as_deref() {
                Some("middle") => 0.5,
                Some("end") => 1.0,
                _ => 0.0,
            },
            drawn: !style.invisible
                && (style.stroke().is_some()
                    || inherited(node, &self.sheet, "fill").is_none_or(|f| f != "none")),
        };
        let span = flat.spans.len();
        let lists = ["x", "y", "dx", "dy"].map(|name| list(node, name, ctx));
        flat.spans.push((flat.chars.len(), 0, lists));
        for child in node.children() {
            if let Some(text) = child.is_text().then(|| child.text()).flatten() {
                let space = child
                    .ancestors()
                    .find_map(|n| n.attribute((NS_XML_URI, "space")));
                for c in text.chars() {
                    flat.push(Char { c, ..template }, space == Some("preserve"));
                }
                continue;
            }
            if !child.is_element() || child.tag_name().namespace() != Some(SVG_NS) {
                continue;
            }
            match child.tag_name().name() {
                "tspan" | "a" => {
                    let inner = style.child(child, &self.sheet, &mut self.report);
                    if !inner.display_none {
                        self.report.note_properties(child);
                        self.flatten(child, layer, ctx, &inner, flat);
                    }
                }
                "tref" => self.report.note("text (tref)"),
                _ => {}
            }
        }
        flat.spans[span].1 = flat.chars.len();
    }

    /// The report label when `node` is not laid out (AC 9): it holds a
    /// `textPath` or its `writing-mode` is vertical.
    fn skipped(&self, node: Node<'_, '_>) -> Option<&'static str> {
        if node
            .descendants()
            .any(|n| n.has_tag_name((SVG_NS, "textPath")))
        {
            return Some("text (textPath)");
        }
        let mode = inherited(node, &self.sheet, "writing-mode").unwrap_or_default();
        (mode.starts_with("tb") || mode.starts_with("vertical")).then_some("text (vertical)")
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

/// The nearest declared value of the inherited property `prop` at `node`
/// or above (`inherit`/`unset` defer to the parent).
fn inherited(node: Node<'_, '_>, sheet: &Sheet, prop: &str) -> Option<String> {
    node.ancestors()
        .filter(|n| n.is_element())
        .filter_map(|n| declared(n, sheet, prop))
        .find(|v| !v.eq_ignore_ascii_case("inherit") && !v.eq_ignore_ascii_case("unset"))
        .map(|v| v.to_ascii_lowercase())
}

/// An [`IGNORED`] value that changes nothing: `normal`, `auto` or zero.
fn neutral(value: &str) -> bool {
    let number = value.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%');
    matches!(value, "normal" | "auto") || number.parse::<f64>().is_ok_and(|n| n == 0.0)
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

    /// AC 5 — `middle` and `end` shift the chunk left by half or all of
    /// its advance width; `start` does not.
    #[test]
    fn text_anchor_shifts_the_chunk() {
        let s = 20.48 / UPEM;
        for (anchor, shift) in [("start", 0.0), ("middle", 455.0), ("end", 910.0)] {
            let es = page(&format!(
                r#"<text x="50" y="50" font-size="20.48" text-anchor="{anchor}">ll</text>"#
            ))
            .entities;
            let (lo, _) = placed_box('l', 50.0 - shift * s, 50.0, s);
            let (_, hi) = placed_box('l', 50.0 - shift * s + 455.0 * s, 50.0, s);
            assert_box(entity_box(&es), (lo, hi));
        }
    }

    /// AC 5 — a `tspan` with its own `x` starts a chunk anchored on its own;
    /// an inherited anchor applies to both.
    #[test]
    fn each_chunk_is_anchored_separately() {
        let es = page(
            r#"<g text-anchor="middle"><text x="10" y="50" font-size="20.48">l<tspan x="60" y="70">l</tspan></text></g>"#,
        )
        .entities;
        let s = 20.48 / UPEM;
        let half = 455.0 * s / 2.0;
        let n = es.len() / 2;
        assert_box(entity_box(&es[..n]), placed_box('l', 10.0 - half, 50.0, s));
        assert_box(entity_box(&es[n..]), placed_box('l', 60.0 - half, 70.0, s));
    }

    /// The entities of each `l` in `es`, in order (every `l` has as many).
    fn per_l(es: &[Entity], count: usize) -> Vec<(Vec2, Vec2)> {
        es.chunks(es.len() / count).map(entity_box).collect()
    }

    /// AC 6 — an `x` list places the first three characters; the fourth
    /// continues by the third's advance.
    #[test]
    fn an_x_list_places_each_character() {
        let es = page(r#"<text x="0 10 20" y="50" font-size="20.48">llll</text>"#).entities;
        let s = 20.48 / UPEM;
        let got = per_l(&es, 4);
        for (i, x) in [0.0, 10.0, 20.0, 20.0 + 455.0 * s].into_iter().enumerate() {
            assert_box(got[i], placed_box('l', x, 50.0, s));
        }
    }

    /// AC 6 — a `dy` list on a `tspan` shifts its characters cumulatively;
    /// an inner `x` overrides the outer list's value for that character.
    #[test]
    fn tspan_lists_shift_and_override() {
        let es = page(
            r#"<text x="0 10 20" y="50" font-size="20.48">l<tspan x="30" dy="5 5">ll</tspan></text>"#,
        )
        .entities;
        let s = 20.48 / UPEM;
        let got = per_l(&es, 3);
        assert_box(got[0], placed_box('l', 0.0, 50.0, s));
        assert_box(got[1], placed_box('l', 30.0, 55.0, s));
        assert_box(got[2], placed_box('l', 20.0, 60.0, s));
    }

    /// AC 7 — whitespace runs collapse to one space across a `tspan`
    /// boundary, and the ends are trimmed.
    #[test]
    fn whitespace_runs_collapse_and_ends_trim() {
        let es = page(
            "<text x=\"10\" y=\"50\" font-size=\"20.48\">  l \n  <tspan>  l  </tspan>\t</text>",
        )
        .entities;
        let s = 20.48 / UPEM;
        let got = per_l(&es, 2);
        assert_box(got[0], placed_box('l', 10.0, 50.0, s));
        assert_box(got[1], placed_box('l', 10.0 + (455.0 + 569.0) * s, 50.0, s));
        let end = page(
            "<text x=\"50\" y=\"50\" font-size=\"20.48\" text-anchor=\"end\">l <tspan> </tspan></text>",
        )
        .entities;
        assert_box(entity_box(&end), placed_box('l', 50.0 - 455.0 * s, 50.0, s));
    }

    /// AC 7 — `xml:space="preserve"` keeps every space, newlines and tabs
    /// becoming spaces.
    #[test]
    fn preserved_whitespace_keeps_every_space() {
        let es = page(
            r#"<g xml:space="preserve"><text x="10" y="50" font-size="20.48"> l&#10;&#9;l</text></g>"#,
        )
        .entities;
        let s = 20.48 / UPEM;
        let got = per_l(&es, 2);
        assert_box(got[0], placed_box('l', 10.0 + 569.0 * s, 50.0, s));
        let second = 10.0 + (569.0 + 455.0 + 2.0 * 569.0) * s;
        assert_box(got[1], placed_box('l', second, 50.0, s));
    }

    fn label(n: usize, label: &str) -> Vec<(String, usize)> {
        vec![(label.to_owned(), n)]
    }

    /// AC 3 — an uninstalled family draws with the default face and is
    /// reported once per text.
    #[test]
    fn a_substituted_family_is_reported_once_per_text() {
        let svg = page(
            r#"<text font-family="Nope" x="10" y="50">ll</text><text font-family="Nope, serif">l</text>"#,
        );
        assert!(!svg.entities.is_empty());
        assert_eq!(svg.report, label(1, "text (font substituted)"));
    }

    /// AC 8 — each unmapped character is counted; it advances by
    /// `.notdef`'s advance (1536) and draws nothing.
    #[test]
    fn missing_glyphs_advance_by_notdef_and_are_counted() {
        let svg = page(r#"<text x="10" y="50" font-size="20.48">éél</text>"#);
        let s = 20.48 / UPEM;
        let want = placed_box('l', 10.0 + 2.0 * 1536.0 * s, 50.0, s);
        assert_box(entity_box(&svg.entities), want);
        assert_eq!(svg.report, label(2, "text (missing glyph)"));
    }

    /// AC 3 — with no font at all the text is skipped and reported.
    #[test]
    fn without_fonts_text_is_skipped() {
        let svg = page_with(r#"<text x="10" y="50">l</text>"#, &FontBook::empty());
        assert!(svg.entities.is_empty());
        assert_eq!(svg.report, label(1, "text (no font)"));
    }

    /// AC 9 — a `textPath` descendant skips the whole text; a `tref` is
    /// skipped and reported.
    #[test]
    fn text_path_skips_the_text_and_tref_is_reported() {
        let svg = page(
            r##"<text x="10" y="50">l<tspan><textPath href="#p">l</textPath></tspan></text>"##,
        );
        assert!(svg.entities.is_empty());
        assert_eq!(svg.report, label(1, "text (textPath)"));
        let svg = page(r##"<text x="10" y="50">l<tref href="#t"/></text>"##);
        assert!(!svg.entities.is_empty());
        assert_eq!(svg.report, label(1, "text (tref)"));
    }

    /// AC 9 — a vertical `writing-mode`, inherited or own, skips the text.
    #[test]
    fn vertical_writing_mode_skips_the_text() {
        let svg = page(
            r#"<text writing-mode="tb" x="10" y="50">l</text><g style="writing-mode: vertical-rl"><text x="10" y="50">l</text></g><text writing-mode="lr" x="10" y="50">l</text>"#,
        );
        assert_eq!(
            svg.entities,
            page(r#"<text x="10" y="50">l</text>"#).entities
        );
        assert_eq!(svg.report, label(2, "text (vertical)"));
    }

    /// AC 9 — `rotate`, `inline-size`, `letter-spacing` and `word-spacing`
    /// are ignored for layout and reported once each per text; `normal`,
    /// `auto` and zero are not reported.
    #[test]
    fn spacing_rotate_and_inline_size_are_ignored_and_reported() {
        let plain = page(r#"<text x="10" y="50" font-size="20.48">l l</text>"#);
        let svg = page(
            r#"<g letter-spacing="2"><text x="10" y="50" font-size="20.48" rotate="30" style="word-spacing: 3px; inline-size: 50">l <tspan letter-spacing="1" rotate="5">l</tspan></text></g>"#,
        );
        assert_eq!(svg.entities, plain.entities);
        let mut report = svg.report;
        report.sort();
        let want = ["inline-size", "letter-spacing", "rotate", "word-spacing"]
            .map(|n| (format!("text ({n})"), 1));
        assert_eq!(report, want);
        let neutral = page(
            r#"<text x="10" y="50" font-size="20.48" letter-spacing="normal" word-spacing="0" inline-size="auto" rotate="0px">l l</text>"#,
        );
        assert!(neutral.report.is_empty(), "{:?}", neutral.report);
    }

    /// The color of each entity's layer.
    fn layer_colors(svg: &ImportedSvg) -> Vec<[u8; 3]> {
        let color = |id| svg.layers.iter().find(|l| l.id == id).unwrap().color;
        svg.entity_layers.iter().map(|&id| color(id)).collect()
    }

    /// AC 10 — the fill color stands in for a missing stroke; a `tspan`'s
    /// own stroke picks its layer.
    #[test]
    fn text_lands_on_its_stroke_or_fill_layer() {
        let svg = page(
            r##"<text x="10" y="50" fill="#ff0000">l<tspan stroke="#0000ff">l</tspan></text>"##,
        );
        let n = svg.entities.len() / 2;
        let colors = layer_colors(&svg);
        assert!(colors[..n].iter().all(|&c| c == [255, 0, 0]), "{colors:?}");
        assert!(colors[n..].iter().all(|&c| c == [0, 0, 255]), "{colors:?}");
    }

    /// AC 10 — an enclosing `data-layer` group wins over the colors.
    #[test]
    fn a_layer_group_owns_its_text() {
        let svg = page(
            r##"<g data-layer="Engrave" stroke="#00ff00"><text x="10" y="50" fill="#ff0000" stroke="#0000ff">l</text></g>"##,
        );
        assert!(!svg.entities.is_empty());
        let engrave = svg.layers.iter().find(|l| l.name == "Engrave").unwrap().id;
        assert!(svg.entity_layers.iter().all(|&id| id == engrave));
    }

    /// AC 10 — `fill="none"` without a stroke paints nothing and imports
    /// nothing; an undeclared fill is black and imports.
    #[test]
    fn unpainted_text_imports_nothing() {
        let svg = page(
            r#"<text x="10" y="50" fill="none">l</text><g fill="none"><text x="10" y="50">l<tspan stroke="none">l</tspan></text></g>"#,
        );
        assert!(svg.entities.is_empty());
        assert!(!page(r#"<text x="10" y="50">l</text>"#).entities.is_empty());
        let stroked = page(r#"<text x="10" y="50" fill="none" stroke="red">l</text>"#);
        assert!(!stroked.entities.is_empty());
    }
}
