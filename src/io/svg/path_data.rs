//! SVG 2 path data (LCV-172): the `d` attribute resolved into absolute
//! segments in SVG coordinates.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::geometry::Vec2;

mod lexer;

/// One drawn segment of a path, absolute, in SVG coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Segment {
    /// `L`, `H`, `V`, or the closing line of `Z` (possibly zero-length).
    Line(Vec2, Vec2),
    /// `A`; the x-axis rotation is dropped, only circular arcs are imported.
    Arc {
        /// The current point before the arc.
        from: Vec2,
        /// The arc's endpoint.
        to: Vec2,
        /// The x radius as written (sign included).
        rx: f64,
        /// The y radius as written (sign included).
        ry: f64,
        /// The large-arc flag.
        large: bool,
        /// The sweep flag.
        sweep: bool,
    },
    /// `C S Q T`: not imported; `label` is the report label (`path C`, …).
    Skipped {
        /// The report label, under the uppercase command.
        label: &'static str,
        /// The endpoint the current point advances to.
        to: Vec2,
    },
}

/// The segments of one `d`, in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct PathData {
    /// Every segment parsed before the first syntax error.
    pub(super) segments: Vec<Segment>,
    /// Whether the data held a syntax error (AC 8).
    pub(super) error: bool,
}

/// Resolve `d` into absolute segments (AC 1, AC 2, AC 8).
pub(super) fn parse_path_data(d: &str) -> PathData {
    let _ = d;
    PathData::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn line(a: [f64; 2], b: [f64; 2]) -> Segment {
        Segment::Line(p(a[0], a[1]), p(b[0], b[1]))
    }

    fn ok(d: &str) -> Vec<Segment> {
        let data = parse_path_data(d);
        assert!(!data.error, "{d:?} is valid");
        data.segments
    }

    #[test]
    fn absolute_commands_resolve() {
        assert_eq!(
            ok("M 1 2 L 5 2 H 9 V 7 Z"),
            [
                line([1.0, 2.0], [5.0, 2.0]),
                line([5.0, 2.0], [9.0, 2.0]),
                line([9.0, 2.0], [9.0, 7.0]),
                line([9.0, 7.0], [1.0, 2.0]),
            ]
        );
    }

    #[test]
    fn relative_commands_resolve_against_the_current_point() {
        assert_eq!(
            ok("m 1 2 l 4 0 h 4 v 5 z"),
            [
                line([1.0, 2.0], [5.0, 2.0]),
                line([5.0, 2.0], [9.0, 2.0]),
                line([9.0, 2.0], [9.0, 7.0]),
                line([9.0, 7.0], [1.0, 2.0]),
            ]
        );
    }

    #[test]
    fn arcs_resolve_absolute_and_relative() {
        let arc = |from, to, rx, ry, large, sweep| Segment::Arc {
            from,
            to,
            rx,
            ry,
            large,
            sweep,
        };
        assert_eq!(
            ok("M 10 0 A 10 10 30 0 1 0 10 a 5 -5 0 1 0 -5 -5"),
            [
                arc(p(10.0, 0.0), p(0.0, 10.0), 10.0, 10.0, false, true),
                arc(p(0.0, 10.0), p(-5.0, 5.0), 5.0, -5.0, true, false),
            ]
        );
    }

    #[test]
    fn implicit_repetition() {
        assert_eq!(
            ok("M 0 0 L 1 1 2 2"),
            [line([0.0, 0.0], [1.0, 1.0]), line([1.0, 1.0], [2.0, 2.0])]
        );
        assert_eq!(ok("M 0 0 1 1"), [line([0.0, 0.0], [1.0, 1.0])]);
        assert_eq!(
            ok("m 1 1 2 2 3 3"),
            [line([1.0, 1.0], [3.0, 3.0]), line([3.0, 3.0], [6.0, 6.0])]
        );
        assert_eq!(
            ok("M0 0h1 2v3"),
            [
                line([0.0, 0.0], [1.0, 0.0]),
                line([1.0, 0.0], [3.0, 0.0]),
                line([3.0, 0.0], [3.0, 3.0]),
            ]
        );
    }

    #[test]
    fn every_subpath_is_read() {
        assert_eq!(
            ok("M 0 0 L 1 0 M 5 5 L 6 5 m 1 1 l 0 1"),
            [
                line([0.0, 0.0], [1.0, 0.0]),
                line([5.0, 5.0], [6.0, 5.0]),
                line([7.0, 6.0], [7.0, 7.0]),
            ]
        );
    }

    #[test]
    fn after_z_a_relative_command_starts_at_the_subpath_start() {
        assert_eq!(
            ok("M 1 1 L 5 1 L 5 5 z l 2 0"),
            [
                line([1.0, 1.0], [5.0, 1.0]),
                line([5.0, 1.0], [5.0, 5.0]),
                line([5.0, 5.0], [1.0, 1.0]),
                line([1.0, 1.0], [3.0, 1.0]),
            ]
        );
    }

    #[test]
    fn curves_advance_to_their_endpoint_under_the_uppercase_label() {
        let skip = |label, x, y| Segment::Skipped { label, to: p(x, y) };
        assert_eq!(
            ok("M 0 0 C 1 1 2 2 3 3 S 4 4 5 5 Q 6 6 7 7 T 8 8 \
                c 1 1 2 2 3 3 s 1 1 1 1 q 1 1 1 1 t 1 1 L 0 0"),
            [
                skip("path C", 3.0, 3.0),
                skip("path S", 5.0, 5.0),
                skip("path Q", 7.0, 7.0),
                skip("path T", 8.0, 8.0),
                skip("path C", 11.0, 11.0),
                skip("path S", 12.0, 12.0),
                skip("path Q", 13.0, 13.0),
                skip("path T", 14.0, 14.0),
                line([14.0, 14.0], [0.0, 0.0]),
            ]
        );
    }
}
