//! Glyph outlines in font units (LCV-179, ADR 0017 §3): lines and
//! quadratic and cubic Béziers from `ttf-parser`, Y up, unscaled. The
//! caller scales them, flips Y and applies the element transform.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::geometry::Vec2;
use ttf_parser::{Face, GlyphId, OutlineBuilder};

/// One segment of a glyph contour, in font units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    /// A straight segment: start, end.
    Line(Vec2, Vec2),
    /// A quadratic Bézier: start, control, end.
    Quad([Vec2; 3]),
    /// A cubic Bézier: start, both controls, end.
    Cubic([Vec2; 4]),
}

/// A character's glyph in one face.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// The horizontal advance in font units.
    pub advance: f64,
    /// Closed contours, each a chain of segments ending on its start.
    pub contours: Vec<Vec<Seg>>,
    /// The face maps no glyph to the character: `advance` is `.notdef`'s
    /// and nothing is drawn (AC 8).
    pub missing: bool,
}

/// The glyph `face` draws for `c` (AC 1, AC 4, AC 8): its advance and
/// outline, or `.notdef`'s advance and no contours when `c` is unmapped.
pub fn glyph(face: &Face<'_>, c: char) -> Glyph {
    let Some(id) = face.glyph_index(c) else {
        return Glyph {
            advance: advance(face, GlyphId(0)),
            contours: Vec::new(),
            missing: true,
        };
    };
    let mut collector = Collector::default();
    // `None` is an empty outline (a space): no contours.
    let _ = face.outline_glyph(id, &mut collector);
    collector.close();
    Glyph {
        advance: advance(face, id),
        contours: collector.contours,
        missing: false,
    }
}

/// `id`'s horizontal advance in font units (0 when the face has none).
fn advance(face: &Face<'_>, id: GlyphId) -> f64 {
    f64::from(face.glyph_hor_advance(id).unwrap_or(0))
}

/// Gathers `ttf-parser` outline calls into closed contours; zero-length
/// lines are dropped.
#[derive(Debug, Default)]
struct Collector {
    contours: Vec<Vec<Seg>>,
    current: Vec<Seg>,
    start: Vec2,
    pen: Vec2,
}

impl Collector {
    fn to(&mut self, seg: Seg, end: Vec2) {
        self.current.push(seg);
        self.pen = end;
    }
}

/// A font-unit point.
fn pt(x: f32, y: f32) -> Vec2 {
    Vec2::new(f64::from(x), f64::from(y))
}

impl OutlineBuilder for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.start = pt(x, y);
        self.pen = self.start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = pt(x, y);
        if p != self.pen {
            self.to(Seg::Line(self.pen, p), p);
        }
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p = pt(x, y);
        self.to(Seg::Quad([self.pen, pt(x1, y1), p]), p);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p = pt(x, y);
        self.to(Seg::Cubic([self.pen, pt(x1, y1), pt(x2, y2), p]), p);
    }

    /// End the current contour, closing it with a line when its end is not
    /// on its start; an empty contour is dropped.
    fn close(&mut self) {
        if self.current.is_empty() {
            return;
        }
        self.line_to(self.start.x as f32, self.start.y as f32);
        self.contours.push(std::mem::take(&mut self.current));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::fonts::tests::book;

    fn glyph_of(c: char) -> Glyph {
        let book = book();
        let (id, _) = book.face("LCV Test Sans", 400, false).unwrap();
        book.with_face(id, |face| glyph(face, c)).unwrap()
    }

    fn ends(seg: &Seg) -> (Vec2, Vec2) {
        match *seg {
            Seg::Line(a, b) => (a, b),
            Seg::Quad([a, _, b]) => (a, b),
            Seg::Cubic([a, _, _, b]) => (a, b),
        }
    }

    /// Each contour is a chain that ends on its start.
    fn assert_closed_chains(g: &Glyph) {
        assert!(!g.contours.is_empty());
        for contour in &g.contours {
            for pair in contour.windows(2) {
                assert_eq!(ends(&pair[0]).1, ends(&pair[1]).0);
            }
            let (first, last) = (contour.first().unwrap(), contour.last().unwrap());
            assert_eq!(ends(last).1, ends(first).0);
            assert!(
                contour
                    .iter()
                    .all(|s| ends(s).0 != ends(s).1 || !matches!(s, Seg::Line(..)))
            );
        }
    }

    /// AC 1 — `l` is closed contours of lines; advance from `hmtx` (455).
    #[test]
    fn l_is_closed_contours_of_lines() {
        let g = glyph_of('l');
        assert_closed_chains(&g);
        assert!(
            g.contours
                .iter()
                .flatten()
                .all(|s| matches!(s, Seg::Line(..)))
        );
        assert_eq!(g.advance.to_bits(), 455.0_f64.to_bits());
        assert!(!g.missing);
    }

    /// AC 1 — `o` holds quadratics; advance 1139.
    #[test]
    fn o_holds_quadratics() {
        let g = glyph_of('o');
        assert_closed_chains(&g);
        assert_eq!(g.contours.len(), 2);
        assert!(
            g.contours
                .iter()
                .flatten()
                .any(|s| matches!(s, Seg::Quad(..)))
        );
        assert_eq!(g.advance.to_bits(), 1139.0_f64.to_bits());
    }

    /// AC 4 — a space advances and draws nothing, and is not missing.
    #[test]
    fn space_advances_without_contours() {
        let g = glyph_of(' ');
        assert!(g.contours.is_empty() && !g.missing);
        assert_eq!(g.advance.to_bits(), 569.0_f64.to_bits());
    }

    /// AC 8 — an unmapped char is missing, with `.notdef`'s advance (1536).
    #[test]
    fn unmapped_char_is_missing_with_notdef_advance() {
        let g = glyph_of('é');
        assert!(g.missing && g.contours.is_empty());
        assert_eq!(g.advance.to_bits(), 1536.0_f64.to_bits());
    }

    /// An open contour is closed with a line; a zero-length line is dropped.
    #[test]
    fn collector_closes_open_contours_and_drops_zero_lines() {
        let mut c = Collector::default();
        c.move_to(0.0, 0.0);
        c.line_to(0.0, 0.0);
        c.line_to(10.0, 0.0);
        c.quad_to(10.0, 10.0, 0.0, 10.0);
        c.move_to(5.0, 5.0);
        c.close();
        let v = |x, y| Vec2::new(x, y);
        let want = vec![
            Seg::Line(v(0.0, 0.0), v(10.0, 0.0)),
            Seg::Quad([v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)]),
            Seg::Line(v(0.0, 10.0), v(0.0, 0.0)),
        ];
        assert_eq!(c.contours, vec![want]);
    }
}
