//! `<polyline>` and `<polygon>` (LCV-174 AC 7, AC 8): `points` read with the
//! path-data number grammar into an equivalent path of lines.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::Shape;
use crate::geometry::Vec2;
use crate::io::svg::import::path::path_entities;
use crate::io::svg::path_data::lexer::Lexer;
use crate::io::svg::path_data::{PathData, Segment};
use crate::io::svg::viewport::Ctx;

/// A `<polyline>`, or with `close` a `<polygon>`, as world lines; an odd
/// count or a syntax error in `points` keeps the pairs before it and adds
/// `data_error` (AC 8). A missing `points` draws nothing.
pub(super) fn poly(
    n: roxmltree::Node<'_, '_>,
    close: bool,
    data_error: &'static str,
    ctx: &Ctx,
    bed_h: f64,
) -> Shape {
    let data = points_path(n.attribute("points").unwrap_or(""), close);
    let (entities, mut notes) = path_entities(&data, ctx, bed_h);
    if data.error {
        notes.push(data_error);
    }
    Shape { entities, notes }
}

/// One line per consecutive pair of points, plus the closing line when
/// `close`; zero-length lines are dropped later by [`path_entities`].
fn points_path(points: &str, close: bool) -> PathData {
    let mut lx = Lexer::new(points);
    let (mut pts, mut error) = (Vec::new(), false);
    while !lx.at_end() {
        let x = lx.number();
        match (x, lx.number()) {
            (Some(x), Some(y)) => pts.push(Vec2::new(x, y)),
            _ => {
                error = true;
                break;
            }
        }
    }
    let mut segments: Vec<Segment> = pts.windows(2).map(|p| Segment::Line(p[0], p[1])).collect();
    if close && pts.len() > 1 {
        segments.push(Segment::Line(pts[pts.len() - 1], pts[0]));
    }
    PathData { segments, error }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The points `data` passes through, in order.
    fn ends(data: &PathData) -> Vec<(f64, f64)> {
        data.segments
            .iter()
            .map(|s| match s {
                Segment::Line(_, b) => (b.x, b.y),
                other => panic!("{other:?}"),
            })
            .collect()
    }

    /// AC 7, AC 8 — separators mix, signs glue, and an odd count or a bad
    /// token stops at the last complete pair; `close` adds the closing line.
    #[test]
    fn points_parse_like_path_numbers_and_stop_at_an_error() {
        let ok = points_path(" 1,2-3 4e0\t5 6 ,", false);
        assert_eq!(ends(&ok), [(-3.0, 4.0), (5.0, 6.0)]);
        assert!(!ok.error);
        for bad in ["1 2 3 4 5", "1 2 3 4 x 6", "1 2 3 4 5 ."] {
            let data = points_path(bad, false);
            assert_eq!(ends(&data), [(3.0, 4.0)], "{bad}");
            assert!(data.error, "{bad}");
        }
        assert_eq!(
            ends(&points_path("1 2 3 4", true)),
            [(3.0, 4.0), (1.0, 2.0)]
        );
        assert!(points_path("1 2", true).segments.is_empty());
    }
}
